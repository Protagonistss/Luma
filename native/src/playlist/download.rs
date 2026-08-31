use crate::error::{AppError, AppResult};
use crate::playlist::{parse_m3u, playlist_from_stream_url, Playlist};

const MAX_DOWNLOAD_BYTES: usize = 5 * 1024 * 1024;
const DOWNLOAD_TIMEOUT_SECS: u64 = 30;
const GH_PROXY_PREFIX: &str = "https://ghfast.top/";

/// GitHub hosts are frequently unreachable from mainland networks without a
/// proxy. Returns mirror URLs to try, in order, when a direct fetch fails.
/// jsDelivr serves a CDN copy of `raw.githubusercontent.com` files; the gh
/// proxy prefix is kept as a second chance when jsDelivr is throttled.
/// GitHub Pages hosts (`*.github.io`) have no reliable mirror and are skipped.
pub(crate) fn mirror_candidates(url: &str) -> Vec<String> {
    let Ok(parsed) = url::Url::parse(url) else {
        return Vec::new();
    };
    if parsed.host_str() != Some("raw.githubusercontent.com") {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    let segments: Vec<&str> = parsed
        .path()
        .trim_start_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    // /user/repo/ref/path -> cdn.jsdelivr.net/gh/user/repo@ref/path
    if segments.len() >= 4 {
        candidates.push(format!(
            "https://cdn.jsdelivr.net/gh/{}/{}@{}/{}",
            segments[0],
            segments[1],
            segments[2],
            segments[3..].join("/")
        ));
    }
    candidates.push(format!("{GH_PROXY_PREFIX}{url}"));
    candidates
}

pub async fn download_playlist(url: &str) -> AppResult<Playlist> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::InvalidPlaylist(
            "playlist URL must use http or https".to_string(),
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .build()?;

    // Direct URL first, then mirrors. Any network-level failure (connect
    // reset, timeout, bad status) falls through to the next candidate so a
    // blocked CDN does not kill the subscription; other errors (oversize,
    // invalid encoding) would repeat on mirrors and abort immediately.
    let mut candidates = vec![url.to_string()];
    candidates.extend(mirror_candidates(url));

    let mut last_network_error: Option<AppError> = None;
    let mut content: Option<String> = None;
    for candidate in &candidates {
        match fetch_body(&client, candidate).await {
            Ok(text) => {
                content = Some(text);
                break;
            }
            Err(err @ AppError::Network(_)) => {
                eprintln!("[luma] playlist fetch failed ({candidate}): {err}");
                last_network_error = Some(err);
            }
            Err(err) => return Err(err),
        }
    }

    let Some(content) = content else {
        return Err(last_network_error
            .unwrap_or_else(|| AppError::Network("download failed".to_string())));
    };

    match parse_m3u(&content) {
        Ok(playlist) => Ok(playlist),
        Err(_) => playlist_from_stream_url(url, None),
    }
}

async fn fetch_body(client: &reqwest::Client, url: &str) -> AppResult<String> {
    let mut response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "download failed with status {}",
            response.status()
        )));
    }

    // Reject oversized playlists from Content-Length before buffering.
    if let Some(length) = response.content_length() {
        ensure_within_limit(0, length as usize)?;
    }

    // Stream the body and abort as soon as the limit is exceeded instead of
    // buffering a huge file first.
    let mut bytes = Vec::with_capacity(response.content_length().unwrap_or(0) as usize);
    while let Some(chunk) = response.chunk().await? {
        ensure_within_limit(bytes.len(), chunk.len())?;
        bytes.extend_from_slice(&chunk);
    }

    String::from_utf8(bytes)
        .map_err(|err| AppError::InvalidPlaylist(format!("playlist is not valid UTF-8: {err}")))
}

fn ensure_within_limit(current: usize, incoming: usize) -> AppResult<()> {
    if current + incoming > MAX_DOWNLOAD_BYTES {
        return Err(AppError::InvalidPlaylist(format!(
            "playlist exceeds max size ({MAX_DOWNLOAD_BYTES} bytes)"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_non_http_url() {
        let err = download_playlist("file:///tmp/test.m3u").await.unwrap_err();
        assert_eq!(err.code(), "INVALID_PLAYLIST");
    }

    #[tokio::test]
    async fn accepts_direct_hls_stream_url() {
        let playlist = download_playlist("https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8")
            .await
            .expect("direct stream url");

        assert_eq!(playlist.channels.len(), 1);
        assert!(playlist.channels[0]
            .stream_url
            .contains("test-streams.mux.dev"));
    }

    #[test]
    fn builds_mirrors_for_github_raw_urls() {
        let candidates = mirror_candidates(
            "https://raw.githubusercontent.com/vbskycn/iptv/master/tv/iptv4.m3u",
        );
        assert_eq!(
            candidates,
            vec![
                "https://cdn.jsdelivr.net/gh/vbskycn/iptv@master/tv/iptv4.m3u".to_string(),
                "https://ghfast.top/https://raw.githubusercontent.com/vbskycn/iptv/master/tv/iptv4.m3u"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn skips_mirrors_for_non_github_raw_hosts() {
        assert!(mirror_candidates("https://example.com/list.m3u").is_empty());
        // GitHub Pages has no reliable mirror; only raw repo files do.
        assert!(mirror_candidates("https://iptv-org.github.io/iptv/countries/cn.m3u").is_empty());
        assert!(mirror_candidates("not a url").is_empty());
    }
}
