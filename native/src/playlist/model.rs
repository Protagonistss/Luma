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
    /// Per-channel request hints carried by many IPTV lists; origins that
    /// enforce User-Agent / Referer checks reject clients without them.
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub referrer: Option<String>,
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
#[serde(rename_all = "camelCase", tag = "type", rename_all_fields = "camelCase")]
pub enum PlaylistSource {
    Url {
        url: String,
        /// Omitted in installs before displayUrl was persisted.
        #[serde(default)]
        display_url: String,
    },
    File { path: String, display_name: String },
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

    /// Backfill fields added after first release so old JSON still loads.
    pub fn normalize_legacy_fields(self) -> Self {
        match self {
            Self::Url { url, display_url } if display_url.is_empty() => Self::from_url(&url),
            other => other,
        }
    }
}

/// One playlist subscription. The merged on-screen playlist is built from
/// every enabled subscription, so channels of the same station from
/// different lists become alternate play lines.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    /// Deterministic id derived from the source URL/path.
    pub id: String,
    pub source: PlaylistSource,
    #[serde(default)]
    pub enabled: bool,
    /// Unix seconds of the last successful fetch; 0 = never fetched.
    #[serde(default)]
    pub imported_at: u64,
}

impl Subscription {
    pub fn from_source(source: PlaylistSource) -> Self {
        let id = match &source {
            PlaylistSource::Url { url, .. } => subscription_id(url),
            PlaylistSource::File { path, .. } => subscription_id(path),
        };
        Self {
            id,
            source,
            enabled: true,
            imported_at: 0,
        }
    }
}

fn subscription_id(seed: &str) -> String {
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, seed.as_bytes()).to_string()
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
