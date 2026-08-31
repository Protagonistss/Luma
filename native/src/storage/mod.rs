use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::playlist::{
    Channel, ChannelGroup, ChannelProbeResult, Playlist, PlaylistSource, ProbeStatus,
};

const PLAYLIST_FILE: &str = "playlist.json";
const SOURCE_FILE: &str = "source.json";
const SUBSCRIPTIONS_FILE: &str = "subscriptions.json";
const SUBSCRIPTION_CACHE_DIR: &str = "subscriptions";
const FAVORITES_FILE: &str = "favorites.json";
const RECENT_FILE: &str = "recent.json";
const SETTINGS_FILE: &str = "settings.json";
const PROBE_FILE: &str = "probe-status.json";
const MAX_RECENT: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// Import-time channel normalization (CN rules first, regions to come).
    pub smart_grouping: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self { smart_grouping: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PersistedState {
    playlist: Option<Playlist>,
    /// Legacy single-source state, kept only to migrate old installs.
    #[serde(default)]
    source: Option<PlaylistSource>,
    #[serde(default)]
    subscriptions: Vec<crate::playlist::Subscription>,
    favorites: Vec<String>,
    recent: Vec<String>,
    #[serde(default)]
    settings: AppSettings,
    /// Last known probe result per channel id. Pruned on import so entries
    /// for channels that disappeared from the list never linger.
    #[serde(default)]
    probe_status: HashMap<String, ProbeStatus>,
}

struct Store {
    data_dir: PathBuf,
    state: PersistedState,
}

/// In-memory state loaded once at startup. Read operations never touch the
/// disk; write operations mutate memory and persist only the affected file.
static STORE: RwLock<Option<Store>> = RwLock::new(None);

pub fn init(data_dir: PathBuf) -> AppResult<()> {
    fs::create_dir_all(&data_dir)?;
    let state = read_state_from_disk(&data_dir)?;
    let mut guard = write_lock()?;
    *guard = Some(Store { data_dir, state });
    Ok(())
}

fn read_lock() -> AppResult<RwLockReadGuard<'static, Option<Store>>> {
    STORE
        .read()
        .map_err(|_| AppError::Storage("failed to lock store".to_string()))
}

fn write_lock() -> AppResult<RwLockWriteGuard<'static, Option<Store>>> {
    STORE
        .write()
        .map_err(|_| AppError::Storage("failed to lock store".to_string()))
}

fn read_state<'a>(
    guard: &'a RwLockReadGuard<'static, Option<Store>>,
) -> AppResult<&'a PersistedState> {
    guard
        .as_ref()
        .map(|store| &store.state)
        .ok_or_else(|| AppError::Storage("store not initialized".to_string()))
}

fn read_state_mut<'a>(
    guard: &'a mut RwLockWriteGuard<'static, Option<Store>>,
) -> AppResult<(&'a mut PersistedState, &'a Path)> {
    guard
        .as_mut()
        .map(|store| (&mut store.state, store.data_dir.as_path()))
        .ok_or_else(|| AppError::Storage("store not initialized".to_string()))
}

fn playlist_of(state: &PersistedState) -> AppResult<&Playlist> {
    state
        .playlist
        .as_ref()
        .ok_or_else(|| AppError::NotFound("no playlist imported".to_string()))
}

fn read_state_from_disk(dir: &Path) -> AppResult<PersistedState> {
    let mut state = PersistedState {
        playlist: read_json_optional(&dir.join(PLAYLIST_FILE))?,
        source: read_json_optional(&dir.join(SOURCE_FILE))?,
        subscriptions: read_json_optional(&dir.join(SUBSCRIPTIONS_FILE))?.unwrap_or_default(),
        favorites: read_json_optional(&dir.join(FAVORITES_FILE))?.unwrap_or_default(),
        recent: read_json_optional(&dir.join(RECENT_FILE))?.unwrap_or_default(),
        settings: read_json_optional(&dir.join(SETTINGS_FILE))?.unwrap_or_default(),
        probe_status: read_json_optional(&dir.join(PROBE_FILE))?.unwrap_or_default(),
    };

    if let Some(source) = state.source.take() {
        state.source = Some(source.normalize_legacy_fields());
    }
    for subscription in &mut state.subscriptions {
        subscription.source = subscription.source.clone().normalize_legacy_fields();
    }

    // Migrate legacy single-source installs: the old `source.json` becomes
    // the first subscription, seeded with the existing playlist as cache so
    // the first rebuild never needs the network.
    if state.subscriptions.is_empty() {
        if let Some(source) = state.source.clone() {
            let mut subscription = crate::playlist::Subscription::from_source(source);
            subscription.imported_at = state
                .playlist
                .as_ref()
                .and_then(|playlist| playlist.imported_at.trim().parse().ok())
                .unwrap_or(0);
            state.subscriptions = vec![subscription];
        }
    }

    Ok(state)
}

fn read_json_optional<T: for<'de> Deserialize<'de>>(path: &Path) -> AppResult<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&content)?))
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let dir = path
        .parent()
        .ok_or_else(|| AppError::Storage("invalid storage path".to_string()))?;
    fs::create_dir_all(dir)?;
    let temp_path = path.with_extension("tmp");
    let content = serde_json::to_string(value)?;
    fs::write(&temp_path, content)?;
    // `fs::rename` atomically replaces an existing destination on both Unix
    // and Windows (MOVEFILE_REPLACE_EXISTING), so no remove/rename window.
    fs::rename(&temp_path, path)?;
    Ok(())
}

/// Persist the merged playlist (built by the commands layer from all
/// enabled subscriptions). Prunes favorites / history / probe results that
/// reference channels no longer present.
pub fn import_playlist(playlist: Playlist) -> AppResult<()> {
    let mut guard = write_lock()?;
    let (state, dir) = read_state_mut(&mut guard)?;

    // Build the next state fully before touching the disk so a failed write
    // leaves both memory and disk untouched.
    let mut next = state.clone();
    next.playlist = Some(playlist);
    next.source = None;

    if let Some(new_playlist) = &next.playlist {
        let ids: HashSet<&str> = new_playlist
            .channels
            .iter()
            .map(|channel| channel.id.as_str())
            .collect();
        next.favorites.retain(|id| ids.contains(id.as_str()));
        next.recent.retain(|id| ids.contains(id.as_str()));
        next.probe_status.retain(|id, _| ids.contains(id.as_str()));
    }

    write_json_atomic(&dir.join(PLAYLIST_FILE), &next.playlist)?;
    write_json_atomic(&dir.join(SUBSCRIPTIONS_FILE), &next.subscriptions)?;
    write_json_atomic(&dir.join(FAVORITES_FILE), &next.favorites)?;
    write_json_atomic(&dir.join(RECENT_FILE), &next.recent)?;
    write_json_atomic(&dir.join(PROBE_FILE), &next.probe_status)?;

    *state = next;
    Ok(())
}

pub fn list_subscriptions() -> AppResult<Vec<crate::playlist::Subscription>> {
    let guard = read_lock()?;
    Ok(read_state(&guard)?.subscriptions.clone())
}

/// Insert or replace a subscription by id. Returns the stored subscription.
pub fn upsert_subscription(
    subscription: crate::playlist::Subscription,
) -> AppResult<crate::playlist::Subscription> {
    let mut guard = write_lock()?;
    let (state, dir) = read_state_mut(&mut guard)?;
    match state
        .subscriptions
        .iter_mut()
        .find(|item| item.id == subscription.id)
    {
        Some(existing) => *existing = subscription.clone(),
        None => state.subscriptions.push(subscription.clone()),
    }
    write_json_atomic(&dir.join(SUBSCRIPTIONS_FILE), &state.subscriptions)?;
    Ok(subscription)
}

pub fn remove_subscription(id: &str) -> AppResult<()> {
    let mut guard = write_lock()?;
    let (state, dir) = read_state_mut(&mut guard)?;
    state.subscriptions.retain(|item| item.id != id);
    write_json_atomic(&dir.join(SUBSCRIPTIONS_FILE), &state.subscriptions)?;
    // Drop the channel cache along with the subscription itself.
    let _ = fs::remove_file(subscription_cache_path(dir, id));
    Ok(())
}

pub fn set_subscription_enabled(id: &str, enabled: bool) -> AppResult<()> {
    let mut guard = write_lock()?;
    let (state, dir) = read_state_mut(&mut guard)?;
    if let Some(item) = state.subscriptions.iter_mut().find(|item| item.id == id) {
        item.enabled = enabled;
    }
    write_json_atomic(&dir.join(SUBSCRIPTIONS_FILE), &state.subscriptions)
}

fn subscription_cache_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(SUBSCRIPTION_CACHE_DIR).join(format!("{id}.json"))
}

/// Cache a subscription's last successfully fetched channels so a failing
/// CDN never wipes the merged playlist.
pub fn set_subscription_cache(id: &str, channels: &[Channel]) -> AppResult<()> {
    let guard = read_lock()?;
    let dir = guard.as_ref().map(|store| store.data_dir.clone());
    drop(guard);
    let Some(dir) = dir else {
        return Err(AppError::Storage("store not initialized".to_string()))
    };
    let owned: Vec<Channel> = channels.to_vec();
    write_json_atomic(&subscription_cache_path(&dir, id), &owned)
}

pub fn get_subscription_cache(id: &str) -> AppResult<Option<Vec<Channel>>> {
    let guard = read_lock()?;
    let dir = guard.as_ref().map(|store| store.data_dir.clone());
    drop(guard);
    let Some(dir) = dir else {
        return Err(AppError::Storage("store not initialized".to_string()))
    };
    Ok(read_json_optional(&subscription_cache_path(&dir, id))?)
}

pub fn set_subscription_imported_at(id: &str, imported_at: u64) -> AppResult<()> {
    let mut guard = write_lock()?;
    let (state, dir) = read_state_mut(&mut guard)?;
    if let Some(item) = state.subscriptions.iter_mut().find(|item| item.id == id) {
        item.imported_at = imported_at;
    }
    write_json_atomic(&dir.join(SUBSCRIPTIONS_FILE), &state.subscriptions)
}

pub fn get_smart_grouping() -> AppResult<bool> {
    let guard = read_lock()?;
    Ok(read_state(&guard)?.settings.smart_grouping)
}

pub fn set_smart_grouping(enabled: bool) -> AppResult<()> {
    let mut guard = write_lock()?;
    let (state, dir) = read_state_mut(&mut guard)?;
    state.settings.smart_grouping = enabled;
    write_json_atomic(&dir.join(SETTINGS_FILE), &state.settings)
}

/// Merge fresh probe results into the persisted per-channel map.
pub fn save_probe_status(results: &[ChannelProbeResult]) -> AppResult<()> {
    let mut guard = write_lock()?;
    let (state, dir) = read_state_mut(&mut guard)?;
    for result in results {
        state.probe_status.insert(result.channel_id.clone(), result.status.clone());
    }
    write_json_atomic(&dir.join(PROBE_FILE), &state.probe_status)
}

pub fn get_probe_status() -> AppResult<HashMap<String, ProbeStatus>> {
    let guard = read_lock()?;
    Ok(read_state(&guard)?.probe_status.clone())
}

/// Unix-seconds import time of the current playlist, if any.
pub fn get_playlist_imported_at() -> AppResult<Option<u64>> {
    let guard = read_lock()?;
    let imported_at = read_state(&guard)?
        .playlist
        .as_ref()
        .map(|playlist| playlist.imported_at.trim().to_string());
    Ok(imported_at
        .and_then(|value| value.parse().ok()))
}

pub fn list_channels(group: Option<String>) -> AppResult<Vec<Channel>> {
    let guard = read_lock()?;
    let state = read_state(&guard)?;
    let playlist = playlist_of(state)?;

    let channels = match group {
        Some(group_name) if group_name != "all" => playlist
            .channels
            .iter()
            .filter(|channel| channel.group == group_name)
            .cloned()
            .collect(),
        _ => playlist.channels.clone(),
    };

    Ok(channels)
}

pub fn list_groups() -> AppResult<Vec<ChannelGroup>> {
    let guard = read_lock()?;
    let state = read_state(&guard)?;
    let playlist = playlist_of(state)?;

    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for channel in &playlist.channels {
        *counts.entry(channel.group.clone()).or_default() += 1;
    }

    Ok(counts
        .into_iter()
        .map(|(name, channel_count)| ChannelGroup {
            name,
            channel_count,
        })
        .collect())
}

pub fn toggle_favorite(channel_id: &str) -> AppResult<bool> {
    let mut guard = write_lock()?;
    let (state, dir) = read_state_mut(&mut guard)?;

    let exists = playlist_of(state)?
        .channels
        .iter()
        .any(|channel| channel.id == channel_id);
    if !exists {
        return Err(AppError::NotFound("channel not found".to_string()));
    }

    let is_favorite = if let Some(pos) = state.favorites.iter().position(|id| id == channel_id) {
        state.favorites.remove(pos);
        false
    } else {
        state.favorites.push(channel_id.to_string());
        true
    };

    write_json_atomic(&dir.join(FAVORITES_FILE), &state.favorites)?;
    Ok(is_favorite)
}

pub fn list_favorites() -> AppResult<Vec<Channel>> {
    let guard = read_lock()?;
    let state = read_state(&guard)?;
    let playlist = playlist_of(state)?;

    let favorite_ids: HashSet<&str> = state.favorites.iter().map(String::as_str).collect();

    Ok(playlist
        .channels
        .iter()
        .filter(|channel| favorite_ids.contains(channel.id.as_str()))
        .cloned()
        .collect())
}

pub fn list_recent() -> AppResult<Vec<Channel>> {
    let guard = read_lock()?;
    let state = read_state(&guard)?;
    let playlist = playlist_of(state)?;

    let channel_map: std::collections::HashMap<_, _> = playlist
        .channels
        .iter()
        .map(|channel| (channel.id.as_str(), channel))
        .collect();

    Ok(state
        .recent
        .iter()
        .filter_map(|id| channel_map.get(id.as_str()).cloned().cloned())
        .collect())
}

pub fn record_recent(channel_id: &str) -> AppResult<()> {
    let mut guard = write_lock()?;
    let (state, dir) = read_state_mut(&mut guard)?;

    let exists = playlist_of(state)?
        .channels
        .iter()
        .any(|channel| channel.id == channel_id);
    if !exists {
        return Err(AppError::NotFound("channel not found".to_string()));
    }

    state.recent.retain(|id| id != channel_id);
    state.recent.insert(0, channel_id.to_string());
    state.recent.truncate(MAX_RECENT);
    write_json_atomic(&dir.join(RECENT_FILE), &state.recent)?;
    Ok(())
}

pub fn get_channel(channel_id: &str) -> AppResult<Channel> {
    let guard = read_lock()?;
    let state = read_state(&guard)?;
    let playlist = playlist_of(state)?;

    playlist
        .channels
        .iter()
        .find(|channel| channel.id == channel_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound("channel not found".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlist::{normalize_playlist, parse_m3u, PlaylistSource};
    use serial_test::serial;

    fn sample_playlist() -> Playlist {
        Playlist {
            channels: vec![
                Channel {
                    id: "ch-1".to_string(),
                    name: "Test".to_string(),
                    stream_url: "https://example.com/live.m3u8".to_string(),
                    group: "News".to_string(),
                    logo: None,
                    tvg_id: None,
                    user_agent: None,
                    referrer: None,
                },
                Channel {
                    id: "ch-2".to_string(),
                    name: "Test 2".to_string(),
                    stream_url: "https://example.com/live2.m3u8".to_string(),
                    group: "News".to_string(),
                    logo: None,
                    tvg_id: None,
                    user_agent: None,
                    referrer: None,
                },
            ],
            imported_at: "0".to_string(),
        }
    }

    #[test]
    #[serial]
    fn smart_grouping_setting_roundtrips_and_persists() {
        let dir = tempfile::tempdir().expect("tempdir");
        init(dir.path().to_path_buf()).expect("init");

        // Default is enabled.
        assert!(get_smart_grouping().expect("get"));

        set_smart_grouping(false).expect("set");
        assert!(!get_smart_grouping().expect("get"));

        // Reload from disk to prove persistence.
        init(dir.path().to_path_buf()).expect("re-init");
        assert!(!get_smart_grouping().expect("get"));
    }

    #[test]
    #[serial]
    fn normalize_and_import_applies_smart_grouping() {
        let dir = tempfile::tempdir().expect("tempdir");
        init(dir.path().to_path_buf()).expect("init");

        let content = "#EXTM3U\n#EXTINF:-1 group-title=\"Undefined\",CCTV-13 新闻 (1080p)\nhttps://example.com/cctv13.m3u8\n#EXTINF:-1 group-title=\"Undefined\",湖南卫视 (2160p)\nhttps://example.com/hntv.m3u8\n";

        import_playlist(normalize_playlist(parse_m3u(content).expect("parse"), true)).expect("import");
        let channels = list_channels(None).expect("list");
        assert_eq!(channels[0].name, "CCTV-13 新闻");
        assert_eq!(channels[0].group, "央视");
        assert_eq!(channels[1].group, "卫视");

        // Disabled → everything passes through untouched.
        import_playlist(normalize_playlist(parse_m3u(content).expect("parse"), false)).expect("import");
        let channels = list_channels(None).expect("list");
        assert_eq!(channels[0].name, "CCTV-13 新闻 (1080p)");
        assert_eq!(channels[0].group, "Undefined");
    }

    #[test]
    #[serial]
    fn subscriptions_crud_and_cache_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        init(dir.path().to_path_buf()).expect("init");

        let sub = crate::playlist::Subscription::from_source(PlaylistSource::from_url(
            "https://example.com/list.m3u",
        ));
        upsert_subscription(sub.clone()).expect("upsert");
        set_subscription_cache(&sub.id, &sample_playlist().channels).expect("cache");
        set_subscription_imported_at(&sub.id, 123).expect("ts");

        let stored = list_subscriptions().expect("list");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, sub.id);
        assert!(stored[0].enabled);
        assert_eq!(stored[0].imported_at, 123);

        let cached = get_subscription_cache(&sub.id).expect("cache read");
        assert_eq!(cached.expect("cached").len(), 2);

        // Re-init proves disk persistence.
        init(dir.path().to_path_buf()).expect("re-init");
        assert_eq!(list_subscriptions().expect("list").len(), 1);

        set_subscription_enabled(&sub.id, false).expect("toggle");
        assert!(!list_subscriptions().expect("list")[0].enabled);

        remove_subscription(&sub.id).expect("remove");
        assert!(list_subscriptions().expect("list").is_empty());
        assert!(get_subscription_cache(&sub.id).expect("cache read").is_none());
    }

    #[test]
    #[serial]
    fn legacy_single_source_migrates_to_subscription() {
        let dir = tempfile::tempdir().expect("tempdir");
        let data = dir.path();

        // Old layout: source.json + playlist.json, no subscriptions.json.
        fs::write(
            data.join("source.json"),
            r#"{"type":"url","url":"https://example.com/old.m3u"}"#,
        )
        .expect("write source");
        fs::write(
            data.join("playlist.json"),
            serde_json::to_string(&sample_playlist()).expect("serialize"),
        )
        .expect("write playlist");

        init(data.to_path_buf()).expect("init");

        let subs = list_subscriptions().expect("list");
        assert_eq!(subs.len(), 1);
        assert!(matches!(
            &subs[0].source,
            PlaylistSource::Url { url, display_url }
                if url == "https://example.com/old.m3u" && display_url == "https://example.com/old.m3u"
        ));
        assert!(subs[0].enabled);
    }

    #[test]
    #[serial]
    fn import_and_list_channels() {
        let dir = tempfile::tempdir().expect("tempdir");
        init(dir.path().to_path_buf()).expect("init");

        import_playlist(sample_playlist())
        .expect("import");

        let channels = list_channels(None).expect("list");
        assert_eq!(channels.len(), 2);

        let groups = list_groups().expect("groups");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].channel_count, 2);
    }

    #[test]
    #[serial]
    fn favorites_persist_across_reinit() {
        let dir = tempfile::tempdir().expect("tempdir");
        init(dir.path().to_path_buf()).expect("init");

        import_playlist(sample_playlist())
        .expect("import");

        assert!(toggle_favorite("ch-1").expect("toggle"));
        assert_eq!(list_favorites().expect("favorites").len(), 1);

        // Re-initialize from the same directory: state must be reloaded from disk.
        init(dir.path().to_path_buf()).expect("re-init");
        assert_eq!(list_favorites().expect("favorites").len(), 1);
        assert!(!toggle_favorite("ch-1").expect("toggle off"));
    }

    #[test]
    #[serial]
    fn reimporting_same_content_keeps_favorites_and_history() {
        let dir = tempfile::tempdir().expect("tempdir");
        init(dir.path().to_path_buf()).expect("init");

        let content = "#EXTM3U\n#EXTINF:-1 group-title=\"央视\",CCTV-1\nhttps://example.com/live/cctv1.m3u8\n#EXTINF:-1 group-title=\"央视\",CCTV-2\nhttps://example.com/live/cctv2.m3u8\n";
        import_playlist(normalize_playlist(parse_m3u(content).expect("parse"), false))
        .expect("import");

        let channels = list_channels(None).expect("channels");
        let cctv1_id = channels
            .iter()
            .find(|c| c.name == "CCTV-1")
            .map(|c| c.id.clone())
            .expect("cctv1");
        assert!(toggle_favorite(&cctv1_id).expect("toggle"));

        // Simulates a daily refresh: same content re-parsed from scratch.
        import_playlist(normalize_playlist(parse_m3u(content).expect("parse"), false))
        .expect("re-import");

        // Deterministic ids keep the favorite attached.
        let favorites = list_favorites().expect("favorites");
        assert_eq!(favorites.len(), 1);
        assert_eq!(favorites[0].name, "CCTV-1");
    }

    #[test]
    #[serial]
    fn probe_status_persists_and_prunes_with_playlist() {
        use crate::playlist::ChannelProbeResult;

        let dir = tempfile::tempdir().expect("tempdir");
        init(dir.path().to_path_buf()).expect("init");

        import_playlist(sample_playlist())
        .expect("import");

        save_probe_status(&[
            ChannelProbeResult {
                channel_id: "ch-1".to_string(),
                status: ProbeStatus::Playable,
                latency_ms: Some(12),
                message: None,
            },
            ChannelProbeResult {
                channel_id: "gone-id".to_string(),
                status: ProbeStatus::Unreachable,
                latency_ms: None,
                message: None,
            },
        ])
        .expect("save");

        let status = get_probe_status().expect("status");
        assert_eq!(status.get("ch-1"), Some(&ProbeStatus::Playable));
        assert!(status.contains_key("gone-id"));

        // Re-import of the same playlist prunes the orphaned entry.
        import_playlist(sample_playlist())
        .expect("re-import");
        let status = get_probe_status().expect("status");
        assert!(status.contains_key("ch-1"));
        assert!(!status.contains_key("gone-id"));

        // Reload from disk to prove persistence.
        init(dir.path().to_path_buf()).expect("re-init");
        assert!(get_probe_status().expect("status").contains_key("ch-1"));
    }

    #[test]
    #[serial]
    fn favorite_for_removed_channel_is_dropped_on_reimport() {
        let dir = tempfile::tempdir().expect("tempdir");
        init(dir.path().to_path_buf()).expect("init");

        import_playlist(sample_playlist())
        .expect("import");
        assert!(toggle_favorite("ch-2").expect("toggle"));

        // Re-import without ch-2.
        let mut smaller = sample_playlist();
        smaller.channels.truncate(1);
        import_playlist(smaller)
        .expect("re-import");

        assert!(list_favorites().expect("favorites").is_empty());
    }

    #[test]
    #[serial]
    fn record_recent_dedupes_and_truncates() {
        let dir = tempfile::tempdir().expect("tempdir");
        init(dir.path().to_path_buf()).expect("init");

        import_playlist(sample_playlist())
        .expect("import");

        record_recent("ch-1").expect("record");
        record_recent("ch-2").expect("record");
        record_recent("ch-1").expect("record again");

        let recent = list_recent().expect("recent");
        assert_eq!(
            recent.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            vec!["ch-1", "ch-2"]
        );
    }
}
