pub mod commands;
pub mod error;
pub mod playlist;
pub mod storage;
pub mod stream_proxy;

use tauri::Manager;

#[cfg(desktop)]
fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    };

    let show_item = MenuItem::with_id(app, "show", "显示 Luma", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let Some(icon) = app.default_window_icon() else {
        return Ok(());
    };

    TrayIconBuilder::new()
        .icon(icon.clone())
        .menu(&menu)
        .tooltip("Luma")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

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
                .map_err(std::io::Error::other)?;
            app.manage(proxy_state);

            #[cfg(desktop)]
            setup_tray(app)?;

            #[cfg(desktop)]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_decorations(false);
            }

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
