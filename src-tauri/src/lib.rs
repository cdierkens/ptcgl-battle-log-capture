pub mod buttons;
pub mod calibration;
pub mod commands;
pub mod coords;
pub mod dedupe;
pub mod engine;
pub mod events;
pub mod gate;
pub mod logging;
pub mod logname;
pub mod logs;
pub mod mode;
pub mod platform;
pub mod ports;
pub mod settings;
pub mod state;
pub mod title;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};
    use tauri::Manager;

    use crate::state::CaptureState;

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::list_logs,
            commands::read_log,
            commands::get_capture_running,
            commands::set_capture_running,
            commands::get_capture_state,
            commands::get_mode,
            commands::set_mode,
        ])
        .setup(|app| {
            let guard = logging::init();
            app.manage(guard);
            let running = Arc::new(AtomicBool::new(true));
            app.manage(running.clone());
            let state_cell = Arc::new(Mutex::new(CaptureState::Idle));
            app.manage(state_cell.clone());
            let handle = app.handle().clone();
            std::thread::spawn(move || engine::run(handle, running, state_cell));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}