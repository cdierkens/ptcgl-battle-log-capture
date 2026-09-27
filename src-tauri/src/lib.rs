pub mod buttons;
pub mod commands;
pub mod coords;
pub mod dedupe;
pub mod engine;
pub mod events;
pub mod gate;
pub mod logging;
pub mod platform;
pub mod ports;
pub mod settings;
pub mod state;
pub mod title;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager;

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
        ])
        .setup(|app| {
            let guard = logging::init();
            app.manage(guard);
            let handle = app.handle().clone();
            std::thread::spawn(move || engine::run(handle));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}