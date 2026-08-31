pub mod commands;
pub mod error;
pub mod playlist;
pub mod storage;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_player::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            storage::init(app_data_dir)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_playlist_from_url,
            commands::import_playlist_from_text,
            commands::refresh_playlist,
            commands::list_channels,
            commands::list_groups,
            commands::toggle_favorite,
            commands::list_favorites,
            commands::list_recent,
            commands::record_recent,
            commands::get_playlist_source,
            commands::play_channel,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
