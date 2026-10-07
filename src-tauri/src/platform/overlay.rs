//! Tauri-backed overlay windows: the capture drawer and the Continue shield.

use std::sync::Mutex;

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::overlay_geometry::{drawer_rect, to_screen, ScreenRect};
use crate::ports::{OverlayError, OverlayPresenter, WindowInfo};

pub const DRAWER_LABEL: &str = "capture-drawer";
pub const SHIELD_LABEL: &str = "continue-shield";

pub struct TauriOverlayPresenter {
    app: AppHandle,
    drawer_game: Mutex<Option<WindowInfo>>,
    open: Mutex<bool>,
}

impl TauriOverlayPresenter {
    /// Creates both overlay windows, hidden. Call from Tauri `setup`.
    pub fn new(app: &AppHandle) -> Result<Self, OverlayError> {
        // The drawer takes keyboard focus when clicked (paste); the shield never needs it.
        build_overlay(app, DRAWER_LABEL, true)?;
        build_overlay(app, SHIELD_LABEL, false)?;
        Ok(Self { app: app.clone(), drawer_game: Mutex::new(None), open: Mutex::new(false) })
    }

    fn window(&self, label: &'static str) -> Result<WebviewWindow, OverlayError> {
        self.app.get_webview_window(label).ok_or(OverlayError::WindowUnavailable { label })
    }

    fn place(&self, label: &'static str, rect: ScreenRect) -> Result<(), OverlayError> {
        let window = self.window(label)?;
        let fail = |e: tauri::Error| OverlayError::Window { label, reason: e.to_string() };
        window.set_size(LogicalSize::new(rect.width, rect.height)).map_err(fail)?;
        window.set_position(LogicalPosition::new(rect.x, rect.y)).map_err(fail)?;
        if window.is_visible().map_err(fail)? {
            return Ok(());
        }
        raise(&window, label)
    }

    fn hide(&self, label: &'static str) -> Result<(), OverlayError> {
        self.window(label)?.hide().map_err(|e| OverlayError::Window { label, reason: e.to_string() })
    }
}

fn poisoned(label: &'static str) -> OverlayError {
    OverlayError::Window { label, reason: "overlay state lock poisoned".into() }
}

impl OverlayPresenter for TauriOverlayPresenter {
    fn show_drawer(&self, game: &WindowInfo) -> Result<(), OverlayError> {
        let open = *self.open.lock().map_err(|_| poisoned(DRAWER_LABEL))?;
        *self.drawer_game.lock().map_err(|_| poisoned(DRAWER_LABEL))? = Some(game.clone());
        self.place(DRAWER_LABEL, drawer_rect(game, open))
    }

    fn set_drawer_open(&self, open: bool) -> Result<(), OverlayError> {
        *self.open.lock().map_err(|_| poisoned(DRAWER_LABEL))? = open;
        let game = self.drawer_game.lock().map_err(|_| poisoned(DRAWER_LABEL))?.clone();
        match game {
            Some(game) => self.place(DRAWER_LABEL, drawer_rect(&game, open)),
            None => Ok(()),
        }
    }

    fn hide_drawer(&self) -> Result<(), OverlayError> {
        *self.drawer_game.lock().map_err(|_| poisoned(DRAWER_LABEL))? = None;
        self.hide(DRAWER_LABEL)
    }

    fn show_shield(&self, game: &WindowInfo, local: ScreenRect) -> Result<(), OverlayError> {
        self.place(SHIELD_LABEL, to_screen(game, local))
    }

    fn hide_shield(&self) -> Result<(), OverlayError> {
        self.hide(SHIELD_LABEL)
    }
}

fn build_overlay(app: &AppHandle, label: &'static str, focusable: bool) -> Result<(), OverlayError> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(label)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .resizable(false)
        .focused(false)
        .focusable(focusable)
        .accept_first_mouse(true)
        .visible_on_all_workspaces(true)
        .visible(false)
        .build()
        .map(|_| ())
        .map_err(|e| OverlayError::Window { label, reason: e.to_string() })
}

/// How long to wait for the main thread to run an AppKit call.
#[cfg(target_os = "macos")]
const MAIN_THREAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// Float above a fullscreen game (status-bar level, every Space, fullscreen
/// auxiliary) and show without making the window key, so the game keeps
/// keyboard focus (Tauri's `show()` is `makeKeyAndOrderFront` on macOS).
/// Waits for the main thread so failures reach the engine and are retried.
#[cfg(target_os = "macos")]
fn raise(window: &WebviewWindow, label: &'static str) -> Result<(), OverlayError> {
    use objc2_app_kit::{NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior};

    let (done, result) = std::sync::mpsc::sync_channel::<Result<(), String>>(1);
    let target = window.clone();
    // Tauri runs this inline when already on the main thread, so waiting cannot deadlock.
    window
        .run_on_main_thread(move || {
            let outcome = target.ns_window().map_err(|e| e.to_string()).map(|ptr| {
                // SAFETY: Tauri returns this window's live NSWindow, and we are on the main thread.
                let ns_window: &NSWindow = unsafe { &*ptr.cast::<NSWindow>() };
                ns_window.setLevel(NSStatusWindowLevel);
                ns_window.setCollectionBehavior(
                    ns_window.collectionBehavior()
                        | NSWindowCollectionBehavior::CanJoinAllSpaces
                        | NSWindowCollectionBehavior::FullScreenAuxiliary,
                );
                ns_window.orderFrontRegardless();
            });
            let _ = done.send(outcome);
        })
        .map_err(|e| OverlayError::Window { label, reason: e.to_string() })?;
    match result.recv_timeout(MAIN_THREAD_TIMEOUT) {
        Ok(outcome) => outcome.map_err(|reason| OverlayError::Window { label, reason }),
        Err(_) => Err(OverlayError::Window { label, reason: "timed out waiting for the main thread".into() }),
    }
}

#[cfg(not(target_os = "macos"))]
fn raise(window: &WebviewWindow, label: &'static str) -> Result<(), OverlayError> {
    window.show().map_err(|e| OverlayError::Window { label, reason: e.to_string() })
}
