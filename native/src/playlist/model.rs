use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub stream_url: String,
    pub group: String,
    pub logo: Option<String>,
    pub tvg_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChannelGroup {
    pub name: String,
    pub channel_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub channels: Vec<Channel>,
    pub imported_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum PlaylistSource {
    Url {
        url: String,
        display_url: String,
    },
    File {
        path: String,
        display_name: String,
    },
}

impl PlaylistSource {
    pub fn from_url(url: &str) -> Self {
        Self::Url {
            url: url.to_string(),
            display_url: mask_credentials(url),
        }
    }

    pub fn from_file(path: &str, display_name: &str) -> Self {
        Self::File {
            path: path.to_string(),
            display_name: display_name.to_string(),
        }
    }
}

pub fn mask_credentials(input: &str) -> String {
    if let Ok(mut parsed) = url::Url::parse(input) {
        if !parsed.username().is_empty() || parsed.password().is_some() {
            let _ = parsed.set_username("");
            let _ = parsed.set_password(None);
            return parsed.to_string();
        }
    }
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::mask_credentials;

    #[test]
    fn masks_url_credentials() {
        let masked = mask_credentials("https://user:secret@example.com/list.m3u");
        assert!(!masked.contains("secret"));
        assert!(!masked.contains("user"));
    }
}
