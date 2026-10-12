pub mod bmp;
pub mod buttons;
pub mod clipboard;
pub mod commands;
pub mod coords;
pub mod dedupe;
pub mod delivery_queue;
pub mod destinations;
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
pub mod sync_status;
pub mod sync_worker;
pub mod title;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use std::sync::{mpsc, Arc, Mutex, RwLock};
    use tauri::Manager;

    use crate::delivery_queue::SharedQueue;
    use crate::overlay_state::OverlayState;
    use crate::platform::keychain::KeychainSecretStore;
    use crate::platform::overlay::TauriOverlayPresenter;
    use crate::platform::permission::ScreenRecordingAccess;
    use crate::platform::store::FsLogStore;
    use crate::ports::{OverlayPresenter, ScreenCaptureAccess, SecretStore};
    use crate::settings::{SettingsPaths, SharedSettings};
    use crate::sink::{LogSink, StoreLogSink, SyncingLogSink};

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
            commands::preview_destination,
            commands::import_destination,
            commands::set_destination_enabled,
            commands::remove_destination,
            commands::open_token_page,
            commands::open_destination_token_page,
            commands::set_destination_token,
            commands::retry_destination,
            commands::get_sync_status,
            commands::sync_log,
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
            let secrets: Arc<dyn SecretStore> = Arc::new(KeychainSecretStore::new(&app.config().identifier));
            app.manage(secrets.clone());
            let queue: SharedQueue = Arc::new(Mutex::new(open_queue(app.handle())?));
            app.manage(queue.clone());
            let sink: Arc<dyn LogSink> = Arc::new(SyncingLogSink::new(
                StoreLogSink::new(FsLogStore::new(shared.clone())),
                shared.clone(),
                queue.clone(),
                results_up(state_cell.clone()),
                sync_changed(app.handle()),
            ));
            app.manage(sink.clone());
            spawn_sync_worker(app.handle(), queue, shared.clone(), secrets, results_up(state_cell.clone()));
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

fn sync_changed(app: &tauri::AppHandle) -> sync_status::SyncChanged {
    let app = app.clone();
    Box::new(move || commands::sync_changed(&app))
}

/// Losing the game window counts as leaving results: the overlay, and its Undo, go with it.
fn results_up(state: std::sync::Arc<std::sync::Mutex<overlay_state::OverlayState>>) -> sink::ResultsUp {
    Box::new(move || {
        matches!(*state.lock().unwrap_or_else(|p| p.into_inner()), overlay_state::OverlayState::Results { .. })
    })
}

/// Releases deliveries held from before a restart on its first tick: no results screen is up yet.
fn spawn_sync_worker(
    app: &tauri::AppHandle,
    queue: delivery_queue::SharedQueue,
    settings: settings::SharedSettings,
    secrets: std::sync::Arc<dyn ports::SecretStore>,
    results_up: sink::ResultsUp,
) {
    let app_version = app.package_info().version.to_string();
    let app = app.clone();
    std::thread::spawn(move || {
        let transport = match platform::http::HttpTransport::new(&app_version) {
            Ok(t) => t,
            Err(e) => return tracing::error!("sync disabled, HTTP client failed to start: {e}"),
        };
        let on_change = sync_changed(&app);
        let transport = Box::new(transport);
        sync_worker::Worker { queue, settings, secrets, transport, results_up, app_version, on_change }.run()
    });
}

/// Falls back to an in-memory queue so a broken sync database never stops logs being saved.
fn open_queue(app: &tauri::AppHandle) -> Result<delivery_queue::DeliveryQueue, delivery_queue::QueueError> {
    use delivery_queue::DeliveryQueue;

    let opened = DeliveryQueue::resolve_path(app)
        .map_err(|e| e.to_string())
        .and_then(|path| DeliveryQueue::open(&path).map_err(|e| e.to_string()));
    opened.or_else(|e| {
        tracing::error!("sync queue unavailable, deliveries won't survive a restart: {e}");
        DeliveryQueue::open_in_memory()
    })
}

fn show_main(app: &tauri::AppHandle) {
    use tauri::Manager;

    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
}
