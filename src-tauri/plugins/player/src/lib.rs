use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenPlayerPayload {
    pub channel_id: String,
    pub name: String,
    pub stream_url: String,
}

#[tauri::command]
async fn open_player(payload: OpenPlayerPayload) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        return Err("native player must be invoked from Android plugin".to_string());
    }

    #[cfg(not(target_os = "android"))]
    {
        let _ = payload;
        Ok(())
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("player")
        .invoke_handler(tauri::generate_handler![open_player])
        .build()
}
