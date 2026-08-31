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

    let (attrs_part, display_name) =
        if let Some((attrs, name)) = split_attrs_and_name(attrs_and_name) {
            (attrs, Some(name.trim().to_string()))
        } else {
            (attrs_and_name, None)
        };

    let attrs = parse_attributes(&strip_duration_prefix(attrs_part));

    Ok(ExtInfMeta {
        display_name: display_name.or_else(|| attrs.get("tvg-name").cloned()),
        group: attrs.get("group-title").cloned(),
        logo: attrs
            .get("tvg-logo")
            .and_then(|value| sanitize_logo_url(value)),
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

fn strip_duration_prefix(input: &str) -> String {
    let trimmed = input.trim();
    let Some(first_char) = trimmed.chars().next() else {
        return String::new();
    };

    if !first_char.is_ascii_digit() && first_char != '-' {
        return trimmed.to_string();
    }

    let Some(space_idx) = trimmed.find(' ') else {
        return String::new();
    };

    trimmed[space_idx..].trim_start().to_string()
}

fn parse_attributes(input: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let mut index = 0;
    let chars: Vec<char> = input.chars().collect();

    while index < chars.len() {
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }
        if index >= chars.len() {
            break;
        }

        let key_start = index;
        while index < chars.len() && chars[index] != '=' {
            index += 1;
        }
        if index >= chars.len() {
            break;
        }

        let key: String = chars[key_start..index].iter().collect();
        index += 1; // skip '='

        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }

        let value = if index < chars.len() && chars[index] == '"' {
            index += 1;
            let value_start = index;
            while index < chars.len() && chars[index] != '"' {
                index += 1;
            }
            let quoted: String = chars[value_start..index].iter().collect();
            if index < chars.len() {
                index += 1; // skip closing quote
            }
            quoted
        } else {
            let value_start = index;
            while index < chars.len() && !chars[index].is_whitespace() {
                index += 1;
            }
            chars[value_start..index].iter().collect()
        };

        if !key.is_empty() {
            attrs.insert(key, value);
        }
    }

    attrs
}

fn is_valid_stream_url(url: &str) -> bool {
    url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("rtmp://")
        || url.starts_with("rtsp://")
}

fn sanitize_logo_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_matches('"');
    let normalized = normalize_http_url(trimmed)?;
    if normalized.contains(".m3u8") {
        return None;
    }
    Some(normalized)
}

fn normalize_http_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_matches('"');
    if trimmed.is_empty() {
        return None;
    }

    let fixed = if let Some(rest) = trimmed.strip_prefix("hhttps://") {
        format!("https://{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("hhttp://") {
        format!("http://{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("http://http://") {
        format!("http://{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("https://https://") {
        format!("https://{rest}")
    } else if trimmed.starts_with("//") {
        format!("https:{trimmed}")
    } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else if looks_like_host(trimmed) {
        format!("https://{trimmed}")
    } else {
        return None;
    };

    url::Url::parse(&fixed).ok().map(|_| fixed)
}

fn looks_like_host(value: &str) -> bool {
    let host = value.split('/').next().unwrap_or(value);
    host.contains('.') && !host.contains(' ')
}

pub fn playlist_from_stream_url(url: &str, name: Option<&str>) -> AppResult<Playlist> {
    if !is_valid_stream_url(url) {
        return Err(AppError::InvalidPlaylist(
            "stream URL must use http, https, rtmp, or rtsp".to_string(),
        ));
    }

    let display_name = name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| derive_name_from_url(url));

    Ok(Playlist {
        channels: vec![Channel {
            id: Uuid::new_v4().to_string(),
            name: display_name,
            stream_url: url.to_string(),
            group: "导入".to_string(),
            logo: None,
            tvg_id: None,
        }],
        imported_at: chrono_now(),
    })
}

fn derive_name_from_url(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|parsed| {
            parsed
                .path_segments()
                .and_then(|segments| segments.filter(|part| !part.is_empty()).next_back())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "直播频道".to_string())
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

    #[test]
    fn parses_logo_attribute_value() {
        let simple_attrs = parse_attributes(r#"tvg-logo="hhttps://i.imgur.com/logo.png""#);
        assert_eq!(
            simple_attrs.get("tvg-logo").map(String::as_str),
            Some("hhttps://i.imgur.com/logo.png")
        );

        let (attrs_part, name) =
            split_attrs_and_name(r#"-1 tvg-logo="hhttps://i.imgur.com/logo.png",Channel A"#)
                .expect("split");
        assert_eq!(name, "Channel A");
        let attrs = parse_attributes(&strip_duration_prefix(attrs_part));
        assert_eq!(
            attrs.get("tvg-logo").map(String::as_str),
            Some("hhttps://i.imgur.com/logo.png")
        );
    }

    #[test]
    fn parses_logo_attribute_from_extinf() {
        let meta = parse_extinf(r#"#EXTINF:-1 tvg-logo="hhttps://i.imgur.com/logo.png",Channel A"#)
            .expect("parse extinf");
        assert_eq!(meta.display_name.as_deref(), Some("Channel A"));
        assert_eq!(meta.logo.as_deref(), Some("https://i.imgur.com/logo.png"));
    }

    #[test]
    fn normalizes_hhttps_logo_url() {
        assert_eq!(
            normalize_http_url("hhttps://i.imgur.com/logo.png").as_deref(),
            Some("https://i.imgur.com/logo.png")
        );
    }

    #[test]
    fn sanitizes_malformed_logo_urls() {
        let content = r#"#EXTM3U
#EXTINF:-1 tvg-logo="hhttps://i.imgur.com/logo.png",Channel A
https://example.com/live/a.m3u8
#EXTINF:-1 tvg-logo="not a url",Channel B
https://example.com/live/b.m3u8
"#;
        let playlist = parse_m3u(content).expect("parse");
        assert_eq!(
            playlist.channels[0].logo.as_deref(),
            Some("https://i.imgur.com/logo.png")
        );
        assert!(playlist.channels[1].logo.is_none());
    }
}
