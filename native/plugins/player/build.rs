const COMMANDS: &[&str] = &["open_player"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
