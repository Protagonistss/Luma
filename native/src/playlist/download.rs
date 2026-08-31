use crate::error::{AppError, AppResult};
use crate::playlist::{parse_m3u, playlist_from_stream_url, Playlist};

const MAX_DOWNLOAD_BYTES: usize = 5 * 1024 * 1024;
const DOWNLOAD_TIMEOUT_SECS: u64 = 30;

pub async fn download_playlist(url: &str) -> AppResult<Playlist> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::InvalidPlaylist(
            "playlist URL must use http or https".to_string(),
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .build()?;

    let response = client.get(url).send().await?;
    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "download failed with status {}",
            response.status()
        )));
    }

    let bytes = response.bytes().await?;
    if bytes.len() > MAX_DOWNLOAD_BYTES {
        return Err(AppError::InvalidPlaylist(format!(
            "playlist exceeds max size ({MAX_DOWNLOAD_BYTES} bytes)"
        )));
    }

    let content = String::from_utf8(bytes.to_vec()).map_err(|err| {
        AppError::InvalidPlaylist(format!("playlist is not valid UTF-8: {err}"))
    })?;

    match parse_m3u(&content) {
        Ok(playlist) => Ok(playlist),
        Err(_) => playlist_from_stream_url(url, None),
    }
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
}
