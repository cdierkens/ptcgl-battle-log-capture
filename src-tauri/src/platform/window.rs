use std::cell::RefCell;
use std::sync::Once;

use tracing::warn;

use crate::ports::{Frame, FrameCapturer, WindowInfo, WindowProvider};

/// Finds the PTCG Live window by title, using `xcap`.
pub struct XcapWindowProvider {
    title_hint: String,
}

impl XcapWindowProvider {
    pub fn new(title_hint: impl Into<String>) -> Self {
        Self { title_hint: title_hint.into() }
    }
}

impl WindowProvider for XcapWindowProvider {
    fn find_game_window(&self) -> Option<WindowInfo> {
        find(&self.title_hint).map(|(_, info)| info)
    }
}

/// Finds the game window and captures it from the same handle, so each poll
/// enumerates windows once. Single-threaded (engine loop only).
pub struct XcapGame {
    title_hint: String,
    found: RefCell<Option<xcap::Window>>,
}

impl XcapGame {
    pub fn new(title_hint: impl Into<String>) -> Self {
        Self { title_hint: title_hint.into(), found: RefCell::new(None) }
    }
}

impl WindowProvider for XcapGame {
    fn find_game_window(&self) -> Option<WindowInfo> {
        let hit = find(&self.title_hint);
        let info = hit.as_ref().map(|(_, info)| info.clone());
        *self.found.borrow_mut() = hit.map(|(window, _)| window);
        info
    }
}

impl FrameCapturer for XcapGame {
    fn capture(&self, _window: &WindowInfo) -> Option<Frame> {
        let found = self.found.borrow();
        let img = found.as_ref()?.capture_image().ok()?;
        Some(Frame { width: img.width(), height: img.height(), rgba: img.into_raw() })
    }
}

fn find(title_hint: &str) -> Option<(xcap::Window, WindowInfo)> {
    let windows = xcap::Window::all().ok()?;
    for window in windows {
        let title = window.title().unwrap_or_default();
        if !crate::title::matches_title(&title, title_hint) {
            continue;
        }
        if window.is_minimized().unwrap_or(false) {
            continue;
        }
        let pid = window.pid().ok();
        if pid.is_none() {
            static WARNED: Once = Once::new();
            WARNED.call_once(|| warn!("game window pid unavailable; overlays show regardless of focus"));
        }
        let info = WindowInfo {
            title,
            pid,
            x: window.x().unwrap_or(0),
            y: window.y().unwrap_or(0),
            width: window.width().unwrap_or(0),
            height: window.height().unwrap_or(0),
        };
        return Some((window, info));
    }
    None
}
