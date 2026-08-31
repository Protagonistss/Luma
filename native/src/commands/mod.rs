use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::error::{AppError, CommandError};
use crate::playlist::{
    download_playlist, normalize_playlist, parse_m3u, Channel, Playlist, PlaylistSource,
    ProbeStatus, Subscription,
};
use crate::storage;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayChannelRequest {
    pub channel_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayChannelResponse {
    pub channel_id: String,
    pub name: String,
    pub stream_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrer: Option<String>,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

async fn fetch_subscription_channels(subscription: &Subscription) -> crate::error::AppResult<Vec<Channel>> {
    match &subscription.source {
        PlaylistSource::Url { url, .. } => Ok(download_playlist(url).await?.channels),
        PlaylistSource::File { path, .. } => {
            let content = tokio::fs::read_to_string(path)
                .await
                .map_err(|err| AppError::File(err.to_string()))?;
            Ok(parse_m3u(&content)?.channels)
        }
    }
}

/// Rebuild the merged playlist from every enabled subscription.
///
/// `force` refetches everything; otherwise a subscription is fetched only
/// when it has never been fetched, or when it is older than `max_age`.
/// Failed fetches fall back to that subscription's cached channels, so one
/// dead CDN never wipes channels from the other subscriptions.
async fn rebuild_merged_playlist(
    force: bool,
    max_age: Option<u64>,
    smart_grouping: bool,
) -> Result<Playlist, CommandError> {
    let subscriptions = storage::list_subscriptions().map_err(CommandError::from)?;
    let now = now_secs();
    let mut channels: Vec<Channel> = Vec::new();
    let mut seen_urls: HashSet<String> = HashSet::new();

    for subscription in subscriptions.iter().filter(|item| item.enabled) {
        let stale = force
            || subscription.imported_at == 0
            || max_age.is_some_and(|age| now.saturating_sub(subscription.imported_at) >= age);

        let fetched = if stale {
            match fetch_subscription_channels(subscription).await {
                Ok(fresh) => {
                    // Cache persistence is best-effort: the merge continues
                    // with the in-memory copy even if the disk write fails.
                    let _ = storage::set_subscription_cache(&subscription.id, &fresh);
                    let _ = storage::set_subscription_imported_at(&subscription.id, now);
                    Some(fresh)
                }
                Err(err) => {
                    eprintln!(
                        "[luma] subscription fetch failed ({}): {err}",
                        subscription.id
                    );
                    None
                }
            }
        } else {
            None
        };

        let sources = fetched
            .or_else(|| storage::get_subscription_cache(&subscription.id).ok().flatten())
            .unwrap_or_default();

        for channel in sources {
            // The same stream may appear in several lists; first one wins.
            if seen_urls.insert(channel.stream_url.clone()) {
                channels.push(channel);
            }
        }
    }

    let merged = Playlist {
        channels,
        imported_at: now.to_string(),
    };
    let normalized = normalize_playlist(merged, smart_grouping);
    storage::set_smart_grouping(smart_grouping).map_err(CommandError::from)?;
    storage::import_playlist(normalized.clone()).map_err(CommandError::from)?;
    Ok(normalized)
}

/// Subscribe to a playlist URL. Fetches it first so a bad URL is rejected
/// without leaving a dead subscription behind; re-adding a known URL forces
/// a fresh fetch of that one subscription.
#[tauri::command]
pub async fn add_subscription_from_url(
    url: String,
    smart_grouping: Option<bool>,
) -> Result<Playlist, CommandError> {
    let flag = smart_grouping.unwrap_or_else(|| storage::get_smart_grouping().unwrap_or(true));
    let mut subscription = Subscription::from_source(PlaylistSource::from_url(&url));

    let channels = fetch_subscription_channels(&subscription)
        .await
        .map_err(CommandError::from)?;

    subscription.imported_at = now_secs();
    storage::upsert_subscription(subscription.clone()).map_err(CommandError::from)?;
    storage::set_subscription_cache(&subscription.id, &channels).map_err(CommandError::from)?;

    rebuild_merged_playlist(false, None, flag).await
}

/// Subscribe to a local file. The caller (frontend) already read the text,
/// so this never touches the network or filesystem.
#[tauri::command]
pub async fn add_subscription_from_file(
    path: String,
    display_name: String,
    content: String,
    smart_grouping: Option<bool>,
) -> Result<Playlist, CommandError> {
    let flag = smart_grouping.unwrap_or_else(|| storage::get_smart_grouping().unwrap_or(true));
    let channels = parse_m3u(&content).map_err(CommandError::from)?.channels;

    let mut subscription =
        Subscription::from_source(PlaylistSource::from_file(&path, &display_name));
    subscription.imported_at = now_secs();
    storage::upsert_subscription(subscription.clone()).map_err(CommandError::from)?;
    storage::set_subscription_cache(&subscription.id, &channels).map_err(CommandError::from)?;

    rebuild_merged_playlist(false, None, flag).await
}

#[tauri::command]
pub fn list_subscriptions() -> Result<Vec<Subscription>, CommandError> {
    storage::list_subscriptions().map_err(CommandError::from)
}

#[tauri::command]
pub async fn remove_subscription(id: String) -> Result<Playlist, CommandError> {
    let smart_grouping = storage::get_smart_grouping().unwrap_or(true);
    storage::remove_subscription(&id).map_err(CommandError::from)?;
    rebuild_merged_playlist(false, None, smart_grouping).await
}

#[tauri::command]
pub async fn toggle_subscription(id: String, enabled: bool) -> Result<Playlist, CommandError> {
    let smart_grouping = storage::get_smart_grouping().unwrap_or(true);
    storage::set_subscription_enabled(&id, enabled).map_err(CommandError::from)?;
    rebuild_merged_playlist(false, None, smart_grouping).await
}

#[tauri::command]
pub async fn refresh_playlist() -> Result<Playlist, CommandError> {
    let smart_grouping = storage::get_smart_grouping().unwrap_or(true);
    let empty = storage::list_subscriptions()
        .map_err(CommandError::from)?
        .iter()
        .all(|item| !item.enabled);
    if empty {
        return Err(CommandError::from(AppError::NotFound(
            "no playlist subscriptions".to_string(),
        )));
    }
    rebuild_merged_playlist(true, None, smart_grouping).await
}

/// Refresh any subscription older than `max_age_secs` (default 24h).
/// Meant for app startup: never fails hard — on any error the existing
/// playlist stays untouched and `None` is returned so the UI loads normally.
#[tauri::command]
pub async fn auto_refresh_playlist(
    max_age_secs: Option<u64>,
) -> Result<Option<Playlist>, CommandError> {
    const DEFAULT_MAX_AGE_SECS: u64 = 24 * 60 * 60;
    let max_age = max_age_secs.unwrap_or(DEFAULT_MAX_AGE_SECS);

    let subscriptions = storage::list_subscriptions().map_err(CommandError::from)?;
    let now = now_secs();
    let any_stale = subscriptions.iter().any(|item| {
        item.enabled && (item.imported_at == 0 || now.saturating_sub(item.imported_at) >= max_age)
    });
    if !any_stale {
        return Ok(None);
    }

    let smart_grouping = storage::get_smart_grouping().unwrap_or(true);
    match rebuild_merged_playlist(false, Some(max_age), smart_grouping).await {
        Ok(playlist) => Ok(Some(playlist)),
        Err(err) => {
            eprintln!("[luma] auto refresh skipped: {}", err.message);
            Ok(None)
        }
    }
}

#[tauri::command]
pub fn get_smart_grouping() -> Result<bool, CommandError> {
    storage::get_smart_grouping().map_err(CommandError::from)
}

#[tauri::command]
pub fn set_smart_grouping(enabled: bool) -> Result<(), CommandError> {
    storage::set_smart_grouping(enabled).map_err(CommandError::from)
}

#[tauri::command]
pub fn get_probe_status() -> Result<HashMap<String, ProbeStatus>, CommandError> {
    storage::get_probe_status().map_err(CommandError::from)
}

#[tauri::command]
pub fn list_channels(group: Option<String>) -> Result<Vec<crate::playlist::Channel>, CommandError> {
    storage::list_channels(group).map_err(CommandError::from)
}

#[tauri::command]
pub fn list_groups() -> Result<Vec<crate::playlist::ChannelGroup>, CommandError> {
    storage::list_groups().map_err(CommandError::from)
}

#[tauri::command]
pub fn toggle_favorite(channel_id: String) -> Result<bool, CommandError> {
    storage::toggle_favorite(&channel_id).map_err(CommandError::from)
}

#[tauri::command]
pub fn list_favorites() -> Result<Vec<crate::playlist::Channel>, CommandError> {
    storage::list_favorites().map_err(CommandError::from)
}

#[tauri::command]
pub fn list_recent() -> Result<Vec<crate::playlist::Channel>, CommandError> {
    storage::list_recent().map_err(CommandError::from)
}

#[tauri::command]
pub fn record_recent(channel_id: String) -> Result<(), CommandError> {
    storage::record_recent(&channel_id).map_err(CommandError::from)
}

#[tauri::command]
pub async fn play_channel(channel_id: String) -> Result<PlayChannelResponse, CommandError> {
    let channel = storage::get_channel(&channel_id).map_err(CommandError::from)?;
    storage::record_recent(&channel_id).map_err(CommandError::from)?;

    Ok(PlayChannelResponse {
        channel_id: channel.id,
        name: channel.name,
        stream_url: channel.stream_url,
        user_agent: channel.user_agent,
        referrer: channel.referrer,
    })
}

#[tauri::command]
pub async fn probe_channels(
    channel_ids: Option<Vec<String>>,
) -> Result<crate::playlist::ProbeReport, CommandError> {
    let channels = if let Some(ids) = channel_ids {
        let mut channels = Vec::new();
        for channel_id in ids {
            channels.push(storage::get_channel(&channel_id).map_err(CommandError::from)?);
        }
        channels
    } else {
        storage::list_channels(None).map_err(CommandError::from)?
    };

    let report = crate::playlist::probe_channels(channels)
        .await
        .map_err(CommandError::from)?;

    // Persist statuses so the next launch (and the “hide unavailable” filter)
    // knows which channels were dead without re-probing everything.
    if let Err(err) = storage::save_probe_status(&report.results) {
        eprintln!("[luma] failed to persist probe status: {err}");
    }

    Ok(report)
}
