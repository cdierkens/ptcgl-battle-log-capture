use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::clipboard::{ClipboardAccess, ClipboardProbe};
use crate::delivery_queue::{self, DeliveryQueue, SharedQueue};
use crate::destinations::{self, Destination, DestinationError, ImportPreview};
use crate::diagnostics::Diagnostics;
use crate::logs::{self, LogSummary};
use crate::overlay_state::{OverlayEvent, OverlayState};
use crate::platform::clipboard::SystemClipboard;
use crate::platform::permission::ScreenRecordingAccess;
use crate::ports::{OverlayPresenter, ScreenCaptureAccess, SecretStore};
use crate::settings::{self, Settings, SettingsPaths, SharedSettings};
use crate::sink::{LogSink, SubmitError, SubmitOutcome};

#[tauri::command]
pub fn is_first_run(paths: State<'_, SettingsPaths>) -> bool {
    !paths.exists()
}

/// Broadcast so every window reflects the latest settings.
pub const SETTINGS_CHANGED_EVENT: &str = "settings://changed";

#[tauri::command]
pub fn get_settings(settings: State<'_, SharedSettings>) -> Settings {
    settings::current(&settings)
}

/// Destinations are left as they are; only the destination commands change them.
/// Returns what was saved.
#[tauri::command]
pub fn save_settings(s: Settings, app: AppHandle) -> Result<Settings, String> {
    let destinations = settings::current(&app.state::<SharedSettings>()).destinations;
    let next = Settings { destinations, ..s };
    apply_settings(&app, next.clone())?;
    Ok(next)
}

/// The one write path for settings (Settings tab, setup flow): persist, share, broadcast.
pub fn apply_settings(app: &AppHandle, next: Settings) -> Result<(), String> {
    app.state::<SettingsPaths>().save(&next)?;
    settings::replace(&app.state::<SharedSettings>(), next.clone());
    if let Err(e) = app.emit(SETTINGS_CHANGED_EVENT, &next) {
        tracing::warn!("failed to emit settings changed: {e}");
    }
    Ok(())
}

/// Runs `change` on the destination list with the keychain and queue, then saves settings.
fn change_destinations<T>(
    app: &AppHandle,
    change: impl FnOnce(&mut Vec<Destination>, &dyn SecretStore, &DeliveryQueue) -> Result<T, DestinationError>,
) -> Result<T, String> {
    let mut next = settings::current(&app.state::<SharedSettings>());
    let secrets = app.state::<Arc<dyn SecretStore>>();
    let shared_queue = app.state::<SharedQueue>();
    let out = {
        let queue = delivery_queue::lock(&shared_queue);
        change(&mut next.destinations, secrets.inner().as_ref(), &queue).map_err(|e| e.to_string())?
    };
    apply_settings(app, next)?;
    Ok(out)
}

/// Validates a pasted destination config for the confirmation step. Saves nothing.
#[tauri::command]
pub fn preview_destination(config: String, settings: State<'_, SharedSettings>) -> Result<ImportPreview, String> {
    let parsed = destinations::parse_config(&config).map_err(|e| e.to_string())?;
    Ok(destinations::preview(&parsed, &settings::current(&settings).destinations))
}

/// Saves a confirmed config; a URL already configured is updated in place.
#[tauri::command]
pub fn import_destination(config: String, token: Option<String>, app: AppHandle) -> Result<(), String> {
    let parsed = destinations::parse_config(&config).map_err(|e| e.to_string())?;
    change_destinations(&app, |list, secrets, queue| {
        destinations::import(list, secrets, queue, parsed, token.as_deref()).map(|_| ())
    })
}

#[tauri::command]
pub fn set_destination_enabled(id: String, enabled: bool, app: AppHandle) -> Result<(), String> {
    change_destinations(&app, |list, _, _| destinations::set_enabled(list, &id, enabled))
}

#[tauri::command]
pub fn remove_destination(id: String, app: AppHandle) -> Result<(), String> {
    change_destinations(&app, |list, secrets, queue| destinations::remove(list, secrets, queue, &id))
}

/// Opens a destination's token page. Only https, so the webview can't launch other schemes.
#[tauri::command]
pub fn open_token_page(url: String, app: AppHandle) -> Result<(), String> {
    let parsed = tauri::Url::parse(&url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "https" {
        return Err("Only https links can be opened.".into());
    }
    app.opener().open_url(parsed.as_str(), None::<&str>).map_err(|e| e.to_string())
}

fn output_dir(settings: &SharedSettings) -> PathBuf {
    PathBuf::from(settings::current(settings).output_dir)
}

#[tauri::command]
pub fn list_logs(settings: State<'_, SharedSettings>) -> Result<Vec<LogSummary>, String> {
    logs::list_logs(&output_dir(&settings))
}

#[tauri::command]
pub fn read_log(file_name: String, settings: State<'_, SharedSettings>) -> Result<String, String> {
    logs::read_log(&output_dir(&settings), &file_name)
}

#[tauri::command]
pub fn get_capture_state(state: State<'_, Arc<Mutex<OverlayState>>>) -> String {
    match state.lock() {
        Ok(s) => s.name().to_string(),
        Err(_) => OverlayState::NoWindow.name().to_string(),
    }
}

#[tauri::command]
pub fn submit_battle_log(
    text: String,
    sink: State<'_, Arc<dyn LogSink>>,
    events: State<'_, Sender<OverlayEvent>>,
) -> Result<SubmitOutcome, SubmitError> {
    let outcome = sink.submit(&text)?;
    if let Some(event) = shield_event(&outcome) {
        // A closed channel means the engine thread has exited; the log is saved regardless.
        let _ = events.send(event);
    }
    Ok(outcome)
}

/// Which overlay event a submit outcome should raise.
fn shield_event(outcome: &SubmitOutcome) -> Option<OverlayEvent> {
    match outcome {
        SubmitOutcome::Saved { .. } => Some(OverlayEvent::LogSaved),
        SubmitOutcome::AlreadySaved => None,
    }
}

#[tauri::command]
pub fn get_clipboard_access() -> ClipboardAccess {
    SystemClipboard.access()
}

/// Reads once, discarding the text, so macOS shows its paste alert now — while the player is
/// turning the setting on — instead of mid-match. Async: off the main thread while it waits.
#[tauri::command]
pub async fn request_clipboard_access() -> ClipboardAccess {
    let _ = SystemClipboard.read_text();
    SystemClipboard.access()
}

#[tauri::command]
pub fn dismiss_shield(events: State<'_, Sender<OverlayEvent>>) {
    let _ = events.send(OverlayEvent::Dismissed);
}

fn trash_and_forget(settings: &SharedSettings, sink: &dyn LogSink, file_name: &str) -> Result<(), String> {
    let text = logs::trash_log(&output_dir(settings), file_name)?;
    sink.forget(&text);
    Ok(())
}

#[tauri::command]
pub fn delete_log(
    file_name: String,
    settings: State<'_, SharedSettings>,
    sink: State<'_, Arc<dyn LogSink>>,
) -> Result<(), String> {
    trash_and_forget(settings.inner(), sink.inner().as_ref(), &file_name)
}

/// Undo the latest save: trash it, forget it, and put the shield back up.
#[tauri::command]
pub fn undo_last_save(
    path: String,
    settings: State<'_, SharedSettings>,
    sink: State<'_, Arc<dyn LogSink>>,
    events: State<'_, Sender<OverlayEvent>>,
) -> Result<(), String> {
    let file_name = logs::file_name_of(&path)?;
    trash_and_forget(settings.inner(), sink.inner().as_ref(), &file_name)?;
    let _ = events.send(OverlayEvent::SaveUndone);
    Ok(())
}

#[tauri::command]
pub fn reveal_log(file_name: String, settings: State<'_, SharedSettings>, app: AppHandle) -> Result<(), String> {
    let path = logs::resolve_log_path(&output_dir(&settings), &file_name)?;
    app.opener().reveal_item_in_dir(path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_logs_folder(settings: State<'_, SharedSettings>, app: AppHandle) -> Result<(), String> {
    let dir = output_dir(&settings);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

const SCREEN_RECORDING_SETTINGS_URL: &str =
    "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture";

#[tauri::command]
pub fn open_screen_recording_settings(app: AppHandle) -> Result<(), String> {
    app.opener().open_url(SCREEN_RECORDING_SETTINGS_URL, None::<&str>).map_err(|e| e.to_string())
}

/// macOS applies a new Screen Recording grant only after a relaunch.
#[tauri::command]
pub fn relaunch_app(app: AppHandle) {
    app.restart();
}

/// Native folder picker starting at the current folder; `None` when cancelled. The UI saves the
/// choice through `save_settings`. Async so the blocking dialog waits off the main thread.
#[tauri::command]
pub async fn pick_output_dir(app: AppHandle) -> Option<String> {
    let current = settings::current(&app.state::<SharedSettings>()).output_dir;
    app.dialog()
        .file()
        .set_directory(current)
        .blocking_pick_folder()
        .and_then(|picked| picked.into_path().ok())
        .map(|path| path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn get_diagnostics(
    app: AppHandle,
    state: State<'_, Arc<Mutex<OverlayState>>>,
    settings: State<'_, SharedSettings>,
) -> String {
    let current = settings::current(&settings);
    Diagnostics {
        version: app.package_info().version.to_string(),
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        state: state.lock().map(|s| s.name().to_string()).unwrap_or_else(|_| "unknown".into()),
        screen_recording: ScreenRecordingAccess.is_authorized(),
        clipboard_access: SystemClipboard.access(),
        prompt_after_match: current.prompt_after_match,
        clipboard_autosave: current.clipboard_autosave,
        saved_logs: logs::list_logs(&PathBuf::from(&current.output_dir)).map(|logs| logs.len()),
    }
    .report()
}

#[tauri::command]
pub fn set_drawer_open(open: bool, presenter: State<'_, Arc<dyn OverlayPresenter>>) -> Result<(), String> {
    presenter.set_drawer_open(open).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_new_save_lowers_the_shield() {
        assert_eq!(shield_event(&SubmitOutcome::Saved { path: "/x".into() }), Some(OverlayEvent::LogSaved));
        // A re-paste (e.g. the previous match's log still on the clipboard) must not.
        assert_eq!(shield_event(&SubmitOutcome::AlreadySaved), None);
    }
}
