use std::collections::HashMap;

use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::playlist::model::{Channel, Playlist};

const MAX_CHANNELS: usize = 10_000;

pub fn parse_m3u(content: &str) -> AppResult<Playlist> {
    let normalized = content.trim_start_matches('\u{feff}');
    if !normalized.starts_with("#EXTM3U") {
        return Err(AppError::InvalidPlaylist(
            "missing #EXTM3U header".to_string(),
        ));
    }

    let mut channels = Vec::new();
    let mut seen_urls = std::collections::HashSet::new();
    let mut pending_meta: Option<ExtInfMeta> = None;

    for raw_line in normalized.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("#EXTINF:") {
            pending_meta = Some(parse_extinf(line)?);
            continue;
        }

        if line.starts_with('#') {
            continue;
        }

        let stream_url = line.to_string();
        if !is_valid_stream_url(&stream_url) {
            pending_meta = None;
            continue;
        }

        if !seen_urls.insert(stream_url.clone()) {
            pending_meta = None;
            continue;
        }

        let meta = pending_meta.take().unwrap_or_default();
        let name = meta
            .display_name
            .clone()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| stream_url.clone());

        channels.push(Channel {
            id: Uuid::new_v4().to_string(),
            name,
            stream_url,
            group: meta.group.unwrap_or_else(|| "未分类".to_string()),
            logo: meta.logo,
            tvg_id: meta.tvg_id,
        });

        if channels.len() > MAX_CHANNELS {
            return Err(AppError::InvalidPlaylist(format!(
                "playlist exceeds max channel limit ({MAX_CHANNELS})"
            )));
        }
    }

    if channels.is_empty() {
        return Err(AppError::InvalidPlaylist(
            "no valid channels found".to_string(),
        ));
    }

    Ok(Playlist {
        channels,
        imported_at: chrono_now(),
    })
}

#[derive(Debug, Default, Clone)]
struct ExtInfMeta {
    display_name: Option<String>,
    group: Option<String>,
    logo: Option<String>,
    tvg_id: Option<String>,
}

fn parse_extinf(line: &str) -> AppResult<ExtInfMeta> {
    let attrs_and_name = line
        .strip_prefix("#EXTINF:")
        .ok_or_else(|| AppError::InvalidPlaylist("invalid #EXTINF line".to_string()))?;

    let (attrs_part, display_name) = if let Some((attrs, name)) = split_attrs_and_name(attrs_and_name)
    {
        (attrs, Some(name.trim().to_string()))
    } else {
        (attrs_and_name, None)
    };

    let attrs = parse_attributes(attrs_part);

    Ok(ExtInfMeta {
        display_name: display_name.or_else(|| attrs.get("tvg-name").cloned()),
        group: attrs.get("group-title").cloned(),
        logo: attrs.get("tvg-logo").cloned(),
        tvg_id: attrs.get("tvg-id").cloned(),
    })
}

fn split_attrs_and_name(input: &str) -> Option<(&str, &str)> {
    let mut last_comma = None;
    let mut in_quotes = false;
    for (idx, ch) in input.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => last_comma = Some(idx),
            _ => {}
        }
    }
    last_comma.map(|idx| (&input[..idx], &input[idx + 1..]))
}

fn parse_attributes(input: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let mut current_key = String::new();
    let mut current_value = String::new();
    let mut in_key = true;
    let mut in_quotes = false;

    for ch in input.chars() {
        match ch {
            '"' if !in_key => in_quotes = !in_quotes,
            '=' if in_key && !in_quotes => in_key = false,
            ' ' if in_key && current_key.is_empty() => {}
            ' ' if !in_key && !in_quotes && current_value.is_empty() => {}
            ' ' if !in_key && !in_quotes => {
                if !current_key.is_empty() {
                    attrs.insert(current_key.clone(), current_value.clone());
                    current_key.clear();
                    current_value.clear();
                    in_key = true;
                }
            }
            _ if in_key => current_key.push(ch),
            _ => current_value.push(ch),
        }
    }

    if !current_key.is_empty() {
        attrs.insert(current_key, current_value);
    }

    attrs
}

fn is_valid_stream_url(url: &str) -> bool {
    url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("rtmp://")
        || url.starts_with("rtsp://")
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    secs.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"#EXTM3U
#EXTINF:-1 tvg-id="cctv1" tvg-name="CCTV-1" tvg-logo="https://example.com/cctv1.png" group-title="央视频道",CCTV-1 综合
https://example.com/live/cctv1.m3u8
#EXTINF:-1 group-title="央视频道",CCTV-2
https://example.com/live/cctv2.m3u8
"#;

    #[test]
    fn parses_basic_m3u() {
        let playlist = parse_m3u(SAMPLE).expect("parse");
        assert_eq!(playlist.channels.len(), 2);
        assert_eq!(playlist.channels[0].name, "CCTV-1 综合");
        assert_eq!(playlist.channels[0].group, "央视频道");
        assert_eq!(
            playlist.channels[0].logo.as_deref(),
            Some("https://example.com/cctv1.png")
        );
    }

    #[test]
    fn parses_bom_and_crlf() {
        let content = format!("\u{feff}{}", SAMPLE.replace('\n', "\r\n"));
        let playlist = parse_m3u(&content).expect("parse");
        assert_eq!(playlist.channels.len(), 2);
    }

    #[test]
    fn rejects_missing_header() {
        let err = parse_m3u("https://example.com/live.m3u8").unwrap_err();
        assert_eq!(err.code(), "INVALID_PLAYLIST");
    }

    #[test]
    fn deduplicates_stream_urls() {
        let content = r#"#EXTM3U
#EXTINF:-1,Channel A
https://example.com/live/a.m3u8
#EXTINF:-1,Channel B
https://example.com/live/a.m3u8
"#;
        let playlist = parse_m3u(content).expect("parse");
        assert_eq!(playlist.channels.len(), 1);
    }

    #[test]
    fn rejects_empty_playlist() {
        let content = "#EXTM3U\n# comment only\n";
        let err = parse_m3u(content).unwrap_err();
        assert_eq!(err.code(), "INVALID_PLAYLIST");
    }
}
