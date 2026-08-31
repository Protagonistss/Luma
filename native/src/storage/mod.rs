use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::playlist::{parse_m3u, Channel, ChannelGroup, Playlist, PlaylistSource};

const PLAYLIST_FILE: &str = "playlist.json";
const SOURCE_FILE: &str = "source.json";
const FAVORITES_FILE: &str = "favorites.json";
const RECENT_FILE: &str = "recent.json";
const MAX_RECENT: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PersistedState {
    playlist: Option<Playlist>,
    source: Option<PlaylistSource>,
    favorites: Vec<String>,
    recent: Vec<String>,
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
    Ok(PersistedState {
        playlist: read_json_optional(&dir.join(PLAYLIST_FILE))?,
        source: read_json_optional(&dir.join(SOURCE_FILE))?,
        favorites: read_json_optional(&dir.join(FAVORITES_FILE))?.unwrap_or_default(),
        recent: read_json_optional(&dir.join(RECENT_FILE))?.unwrap_or_default(),
    })
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

pub fn import_playlist(playlist: Playlist, source: PlaylistSource) -> AppResult<()> {
    let mut guard = write_lock()?;
    let (state, dir) = read_state_mut(&mut guard)?;

    // Build the next state fully before touching the disk so a failed write
    // leaves both memory and disk untouched.
    let mut next = state.clone();
    next.playlist = Some(playlist);
    next.source = Some(source);

    if let Some(new_playlist) = &next.playlist {
        let ids: HashSet<&str> = new_playlist
            .channels
            .iter()
            .map(|channel| channel.id.as_str())
            .collect();
        next.favorites.retain(|id| ids.contains(id.as_str()));
        next.recent.retain(|id| ids.contains(id.as_str()));
    }

    write_json_atomic(&dir.join(PLAYLIST_FILE), &next.playlist)?;
    write_json_atomic(&dir.join(SOURCE_FILE), &next.source)?;
    write_json_atomic(&dir.join(FAVORITES_FILE), &next.favorites)?;
    write_json_atomic(&dir.join(RECENT_FILE), &next.recent)?;

    *state = next;
    Ok(())
}

pub fn import_playlist_from_text(content: &str, source: PlaylistSource) -> AppResult<Playlist> {
    let playlist = parse_m3u(content)?;
    import_playlist(playlist.clone(), source)?;
    Ok(playlist)
}

pub fn get_playlist_source() -> AppResult<Option<PlaylistSource>> {
    let guard = read_lock()?;
    Ok(read_state(&guard)?.source.clone())
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
    use crate::playlist::PlaylistSource;
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
                },
                Channel {
                    id: "ch-2".to_string(),
                    name: "Test 2".to_string(),
                    stream_url: "https://example.com/live2.m3u8".to_string(),
                    group: "News".to_string(),
                    logo: None,
                    tvg_id: None,
                },
            ],
            imported_at: "0".to_string(),
        }
    }

    #[test]
    #[serial]
    fn import_and_list_channels() {
        let dir = tempfile::tempdir().expect("tempdir");
        init(dir.path().to_path_buf()).expect("init");

        import_playlist(
            sample_playlist(),
            PlaylistSource::from_url("https://example.com/list.m3u"),
        )
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

        import_playlist(
            sample_playlist(),
            PlaylistSource::from_url("https://example.com/list.m3u"),
        )
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
    fn favorite_for_removed_channel_is_dropped_on_reimport() {
        let dir = tempfile::tempdir().expect("tempdir");
        init(dir.path().to_path_buf()).expect("init");

        import_playlist(
            sample_playlist(),
            PlaylistSource::from_url("https://example.com/list.m3u"),
        )
        .expect("import");
        assert!(toggle_favorite("ch-2").expect("toggle"));

        // Re-import without ch-2.
        let mut smaller = sample_playlist();
        smaller.channels.truncate(1);
        import_playlist(
            smaller,
            PlaylistSource::from_url("https://example.com/list.m3u"),
        )
        .expect("re-import");

        assert!(list_favorites().expect("favorites").is_empty());
    }

    #[test]
    #[serial]
    fn record_recent_dedupes_and_truncates() {
        let dir = tempfile::tempdir().expect("tempdir");
        init(dir.path().to_path_buf()).expect("init");

        import_playlist(
            sample_playlist(),
            PlaylistSource::from_url("https://example.com/list.m3u"),
        )
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
