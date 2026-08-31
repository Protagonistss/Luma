use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::playlist::{parse_m3u, Channel, ChannelGroup, Playlist, PlaylistSource};

static DATA_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

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

pub fn init(data_dir: PathBuf) -> AppResult<()> {
    fs::create_dir_all(&data_dir)?;
    let mut guard = DATA_DIR
        .lock()
        .map_err(|_| AppError::Storage("failed to lock data dir".to_string()))?;
    *guard = Some(data_dir);
    Ok(())
}

fn data_dir() -> AppResult<PathBuf> {
    DATA_DIR
        .lock()
        .map_err(|_| AppError::Storage("failed to lock data dir".to_string()))?
        .clone()
        .ok_or_else(|| AppError::Storage("data directory not initialized".to_string()))
}

fn read_state() -> AppResult<PersistedState> {
    let dir = data_dir()?;
    let playlist_path = dir.join(PLAYLIST_FILE);
    let source_path = dir.join(SOURCE_FILE);
    let favorites_path = dir.join(FAVORITES_FILE);
    let recent_path = dir.join(RECENT_FILE);

    let playlist = read_json_optional::<Playlist>(&playlist_path)?;
    let source = read_json_optional::<PlaylistSource>(&source_path)?;
    let favorites = read_json_optional::<Vec<String>>(&favorites_path)?.unwrap_or_default();
    let recent = read_json_optional::<Vec<String>>(&recent_path)?.unwrap_or_default();

    Ok(PersistedState {
        playlist,
        source,
        favorites,
        recent,
    })
}

fn write_state(state: &PersistedState) -> AppResult<()> {
    let dir = data_dir()?;
    if let Some(playlist) = &state.playlist {
        write_json_atomic(&dir.join(PLAYLIST_FILE), playlist)?;
    }
    if let Some(source) = &state.source {
        write_json_atomic(&dir.join(SOURCE_FILE), source)?;
    }
    write_json_atomic(&dir.join(FAVORITES_FILE), &state.favorites)?;
    write_json_atomic(&dir.join(RECENT_FILE), &state.recent)?;
    Ok(())
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
    let content = serde_json::to_string_pretty(value)?;
    fs::write(&temp_path, content)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temp_path, path)?;
    Ok(())
}

pub fn import_playlist(playlist: Playlist, source: PlaylistSource) -> AppResult<()> {
    let mut state = read_state()?;
    let backup = state.clone();

    state.playlist = Some(playlist);
    state.source = Some(source);
    state.favorites.retain(|id| {
        state
            .playlist
            .as_ref()
            .map(|p| p.channels.iter().any(|c| c.id == *id))
            .unwrap_or(false)
    });
    state.recent.retain(|id| {
        state
            .playlist
            .as_ref()
            .map(|p| p.channels.iter().any(|c| c.id == *id))
            .unwrap_or(false)
    });

    if let Err(err) = write_state(&state) {
        let _ = write_state(&backup);
        return Err(err);
    }

    Ok(())
}

pub fn import_playlist_from_text(content: &str, source: PlaylistSource) -> AppResult<Playlist> {
    let playlist = parse_m3u(content)?;
    import_playlist(playlist.clone(), source)?;
    Ok(playlist)
}

pub fn get_playlist_source() -> AppResult<Option<PlaylistSource>> {
    Ok(read_state()?.source)
}

pub fn list_channels(group: Option<String>) -> AppResult<Vec<Channel>> {
    let state = read_state()?;
    let playlist = state
        .playlist
        .ok_or_else(|| AppError::NotFound("no playlist imported".to_string()))?;

    let channels = match group {
        Some(group_name) if group_name != "all" => playlist
            .channels
            .into_iter()
            .filter(|c| c.group == group_name)
            .collect(),
        _ => playlist.channels,
    };

    Ok(channels)
}

pub fn list_groups() -> AppResult<Vec<ChannelGroup>> {
    let state = read_state()?;
    let playlist = state
        .playlist
        .ok_or_else(|| AppError::NotFound("no playlist imported".to_string()))?;

    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for channel in &playlist.channels {
        *counts.entry(channel.group.clone()).or_default() += 1;
    }

    Ok(counts
        .into_iter()
        .map(|(name, channel_count)| ChannelGroup { name, channel_count })
        .collect())
}

pub fn toggle_favorite(channel_id: &str) -> AppResult<bool> {
    let mut state = read_state()?;
    let exists = state
        .playlist
        .as_ref()
        .and_then(|p| p.channels.iter().find(|c| c.id == channel_id))
        .is_some();

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

    write_state(&state)?;
    Ok(is_favorite)
}

pub fn list_favorites() -> AppResult<Vec<Channel>> {
    let state = read_state()?;
    let playlist = state
        .playlist
        .ok_or_else(|| AppError::NotFound("no playlist imported".to_string()))?;

    Ok(playlist
        .channels
        .into_iter()
        .filter(|c| state.favorites.contains(&c.id))
        .collect())
}

pub fn list_recent() -> AppResult<Vec<Channel>> {
    let state = read_state()?;
    let playlist = state
        .playlist
        .ok_or_else(|| AppError::NotFound("no playlist imported".to_string()))?;

    let channel_map: std::collections::HashMap<_, _> =
        playlist.channels.into_iter().map(|c| (c.id.clone(), c)).collect();

    Ok(state
        .recent
        .iter()
        .filter_map(|id| channel_map.get(id).cloned())
        .collect())
}

pub fn record_recent(channel_id: &str) -> AppResult<()> {
    let mut state = read_state()?;
    let exists = state
        .playlist
        .as_ref()
        .and_then(|p| p.channels.iter().find(|c| c.id == channel_id))
        .is_some();

    if !exists {
        return Err(AppError::NotFound("channel not found".to_string()));
    }

    state.recent.retain(|id| id != channel_id);
    state.recent.insert(0, channel_id.to_string());
    state.recent.truncate(MAX_RECENT);
    write_state(&state)?;
    Ok(())
}

pub fn get_channel(channel_id: &str) -> AppResult<Channel> {
    let state = read_state()?;
    let playlist = state
        .playlist
        .ok_or_else(|| AppError::NotFound("no playlist imported".to_string()))?;

    playlist
        .channels
        .into_iter()
        .find(|c| c.id == channel_id)
        .ok_or_else(|| AppError::NotFound("channel not found".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playlist::model::Channel;

    fn sample_playlist() -> Playlist {
        Playlist {
            channels: vec![Channel {
                id: "ch-1".to_string(),
                name: "Test".to_string(),
                stream_url: "https://example.com/live.m3u8".to_string(),
                group: "News".to_string(),
                logo: None,
                tvg_id: None,
            }],
            imported_at: "0".to_string(),
        }
    }

    #[test]
    fn import_and_list_channels() {
        let dir = tempfile::tempdir().expect("tempdir");
        init(dir.path().to_path_buf()).expect("init");

        import_playlist(
            sample_playlist(),
            PlaylistSource::from_url("https://example.com/list.m3u"),
        )
        .expect("import");

        let channels = list_channels(None).expect("list");
        assert_eq!(channels.len(), 1);
    }
}
