use serde::{Deserialize, Serialize};

use crate::error::{AppError, CommandError};
use crate::playlist::{download_playlist, Playlist, PlaylistSource};
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
}

#[tauri::command]
pub async fn import_playlist_from_url(url: String) -> Result<Playlist, CommandError> {
    let playlist = download_playlist(&url).await.map_err(CommandError::from)?;
    storage::import_playlist(playlist.clone(), PlaylistSource::from_url(&url))
        .map_err(CommandError::from)?;
    Ok(playlist)
}

#[tauri::command]
pub async fn import_playlist_from_text(
    content: String,
    source: PlaylistSource,
) -> Result<Playlist, CommandError> {
    storage::import_playlist_from_text(&content, source).map_err(CommandError::from)
}

#[tauri::command]
pub async fn refresh_playlist() -> Result<Playlist, CommandError> {
    let source = storage::get_playlist_source()
        .map_err(CommandError::from)?
        .ok_or_else(|| CommandError::from(AppError::NotFound("no playlist source".to_string())))?;

    match source {
        PlaylistSource::Url { url, .. } => import_playlist_from_url(url).await,
        PlaylistSource::File { path, display_name } => {
            let content = tokio::fs::read_to_string(&path)
                .await
                .map_err(|err| CommandError::from(AppError::File(err.to_string())))?;
            import_playlist_from_text(content, PlaylistSource::from_file(&path, &display_name))
                .await
        }
    }
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
pub fn get_playlist_source() -> Result<Option<PlaylistSource>, CommandError> {
    storage::get_playlist_source().map_err(CommandError::from)
}

#[tauri::command]
pub async fn play_channel(channel_id: String) -> Result<PlayChannelResponse, CommandError> {
    let channel = storage::get_channel(&channel_id).map_err(CommandError::from)?;
    storage::record_recent(&channel_id).map_err(CommandError::from)?;

    Ok(PlayChannelResponse {
        channel_id: channel.id,
        name: channel.name,
        stream_url: channel.stream_url,
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

    crate::playlist::probe_channels(channels)
        .await
        .map_err(CommandError::from)
}
