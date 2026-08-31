use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::playlist::Channel;

const PROBE_TIMEOUT_SECS: u64 = 8;
const MAX_PROBE_BODY_BYTES: usize = 8_192;
const MAX_CONCURRENT_PROBES: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProbeStatus {
    Playable,
    Unreachable,
    InvalidBody,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChannelProbeResult {
    pub channel_id: String,
    pub status: ProbeStatus,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProbeReport {
    pub total: usize,
    pub playable: usize,
    pub unreachable: usize,
    pub invalid: usize,
    pub results: Vec<ChannelProbeResult>,
}

pub async fn probe_channels(channels: Vec<Channel>) -> AppResult<ProbeReport> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(PROBE_TIMEOUT_SECS))
        .user_agent("Luma/0.1")
        .build()?;

    let mut results = Vec::with_capacity(channels.len());
    let mut index = 0usize;

    while index < channels.len() {
        let batch = channels[index..channels.len().min(index + MAX_CONCURRENT_PROBES)].to_vec();
        let mut tasks = Vec::with_capacity(batch.len());

        for channel in batch {
            let client = client.clone();
            tasks.push(tokio::spawn(async move {
                let started = Instant::now();
                let (status, message) = probe_stream(&client, &channel).await;
                ChannelProbeResult {
                    channel_id: channel.id,
                    status,
                    latency_ms: Some(started.elapsed().as_millis() as u64),
                    message,
                }
            }));
        }

        for task in tasks {
            let result = task
                .await
                .map_err(|err| AppError::Network(err.to_string()))?;
            results.push(result);
        }

        index += MAX_CONCURRENT_PROBES;
    }

    let playable = results
        .iter()
        .filter(|item| item.status == ProbeStatus::Playable)
        .count();
    let unreachable = results
        .iter()
        .filter(|item| item.status == ProbeStatus::Unreachable)
        .count();
    let invalid = results
        .iter()
        .filter(|item| item.status == ProbeStatus::InvalidBody)
        .count();

    Ok(ProbeReport {
        total: results.len(),
        playable,
        unreachable,
        invalid,
        results,
    })
}

async fn probe_stream(client: &reqwest::Client, channel: &Channel) -> (ProbeStatus, Option<String>) {
    if !channel.stream_url.starts_with("http://") && !channel.stream_url.starts_with("https://") {
        return (
            ProbeStatus::InvalidBody,
            Some("unsupported stream scheme".to_string()),
        );
    }

    // Honor the list's request hints: UA-checked origins reject the default
    // client and would otherwise be misreported as dead.
    let mut request = client.get(&channel.stream_url);
    if let Some(user_agent) = channel.user_agent.as_deref() {
        request = request.header(reqwest::header::USER_AGENT, user_agent);
    }
    if let Some(referrer) = channel.referrer.as_deref() {
        request = request.header(reqwest::header::REFERER, referrer);
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(err) => {
            return (ProbeStatus::Unreachable, Some(err.to_string()));
        }
    };

    if !response.status().is_success() {
        return (
            ProbeStatus::Unreachable,
            Some(format!("http {}", response.status())),
        );
    }

    let bytes = match response.bytes().await {
        Ok(bytes) => bytes,
        Err(err) => {
            return (ProbeStatus::Unreachable, Some(err.to_string()));
        }
    };

    let sample = &bytes[..bytes.len().min(MAX_PROBE_BODY_BYTES)];
    if is_playable_body(sample, &channel.stream_url) {
        (ProbeStatus::Playable, None)
    } else {
        (
            ProbeStatus::InvalidBody,
            Some("response is not a valid HLS playlist".to_string()),
        )
    }
}

fn is_playable_body(body: &[u8], url: &str) -> bool {
    let Ok(text) = std::str::from_utf8(body) else {
        return !url.contains(".m3u8");
    };

    if text.contains("#EXTM3U")
        || text.contains("#EXT-X-STREAM-INF")
        || text.contains("#EXT-X-TARGETDURATION")
    {
        return true;
    }

    url.contains(".m3u8") && text.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_hls_manifest_body() {
        let body = b"#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1280000\nstream.m3u8\n";
        assert!(is_playable_body(body, "http://example.com/master.m3u8"));
    }

    #[test]
    fn rejects_html_body() {
        let body = b"<html><body>forbidden</body></html>";
        assert!(!is_playable_body(
            body,
            "http://example.com/live/cctv1.m3u8"
        ));
    }
}
