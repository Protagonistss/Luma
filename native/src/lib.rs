pub mod commands;
pub mod error;
pub mod playlist;
pub mod storage;
pub mod stream_proxy;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_player::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            storage::init(app_data_dir)?;

            let proxy_state = tauri::async_runtime::block_on(stream_proxy::start_server())
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))?;
            app.manage(proxy_state);

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
            commands::probe_channels,
            stream_proxy::get_proxied_stream_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
