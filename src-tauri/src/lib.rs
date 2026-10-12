pub mod bmp;
pub mod buttons;
pub mod clipboard;
pub mod commands;
pub mod coords;
pub mod dedupe;
pub mod delivery_queue;
pub mod detection;
pub mod diagnostics;
pub mod engine;
pub mod events;
pub mod focus;
pub mod gate;
pub mod logging;
pub mod logname;
pub mod logs;
pub mod overlay_geometry;
pub mod overlay_state;
pub mod platform;
pub mod ports;
pub mod settings;
pub mod sink;
pub mod title;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use std::sync::{mpsc, Arc, Mutex, RwLock};
    use tauri::Manager;

    use crate::overlay_state::OverlayState;
    use crate::platform::overlay::TauriOverlayPresenter;
    use crate::platform::permission::ScreenRecordingAccess;
    use crate::platform::store::FsLogStore;
    use crate::ports::{OverlayPresenter, ScreenCaptureAccess};
    use crate::settings::{SettingsPaths, SharedSettings};
    use crate::sink::{LogSink, StoreLogSink};

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::list_logs,
            commands::read_log,
            commands::get_capture_state,
            commands::submit_battle_log,
            commands::dismiss_shield,
            commands::get_clipboard_access,
            commands::request_clipboard_access,
            commands::delete_log,
            commands::undo_last_save,
            commands::reveal_log,
            commands::open_logs_folder,
            commands::open_screen_recording_settings,
            commands::relaunch_app,
            commands::pick_output_dir,
            commands::get_diagnostics,
            commands::is_first_run,
            commands::set_drawer_open,
        ])
        // The drawer is the everyday UI; closing the main window only hides it (⌘Q quits).
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            let paths = SettingsPaths::resolve(app.handle())?;
            let guard = logging::init(&paths.log_folder.join("logs"));
            app.manage(guard);
            let state_cell = Arc::new(Mutex::new(OverlayState::NoWindow));
            app.manage(state_cell.clone());
            let presenter: Arc<dyn OverlayPresenter> = Arc::new(TauriOverlayPresenter::new(app.handle())?);
            app.manage(presenter.clone());
            if !paths.exists() || !ScreenRecordingAccess.is_authorized() {
                show_main(app.handle());
            }
            let shared: SharedSettings = Arc::new(RwLock::new(paths.load()));
            app.manage(paths);
            app.manage(shared.clone());
            let sink: Arc<dyn LogSink> = Arc::new(StoreLogSink::new(FsLogStore::new(shared.clone())));
            app.manage(sink.clone());
            let (events, inbox) = mpsc::channel();
            app.manage(events);
            let handle = app.handle().clone();
            std::thread::spawn(move || engine::run(handle, state_cell, presenter, inbox, shared, sink));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                show_main(_app);
            }
        });
}

fn show_main(app: &tauri::AppHandle) {
    use tauri::Manager;

    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
}
