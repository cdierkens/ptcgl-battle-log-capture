use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::State;

use crate::logs::{self, LogSummary};
use crate::settings::{self, Settings};
use crate::state::CaptureState;

#[tauri::command]
pub fn get_settings() -> Settings {
    settings::load()
}

#[tauri::command]
pub fn save_settings(s: Settings) -> Result<(), String> {
    settings::save(&s)
}

#[tauri::command]
pub fn list_logs() -> Result<Vec<LogSummary>, String> {
    logs::list_logs(&PathBuf::from(&settings::load().output_dir))
}

#[tauri::command]
pub fn read_log(file_name: String) -> Result<String, String> {
    logs::read_log(&PathBuf::from(&settings::load().output_dir), &file_name)
}

#[tauri::command]
pub fn get_capture_running(running: State<'_, Arc<AtomicBool>>) -> bool {
    running.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_capture_running(running: bool, flag: State<'_, Arc<AtomicBool>>) {
    flag.store(running, Ordering::Relaxed);
}

#[tauri::command]
pub fn get_capture_state(state: State<'_, Arc<Mutex<CaptureState>>>) -> String {
    match state.lock() {
        Ok(s) => s.to_string(),
        Err(_) => "Idle".to_string(),
    }
}