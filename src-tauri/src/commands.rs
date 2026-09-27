use crate::settings::{self, Settings};

#[tauri::command]
pub fn get_settings() -> Settings {
    settings::load()
}

#[tauri::command]
pub fn save_settings(s: Settings) -> Result<(), String> {
    settings::save(&s)
}