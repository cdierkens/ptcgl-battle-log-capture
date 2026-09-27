use crate::ports::{WindowInfo, WindowProvider};

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
        let windows = xcap::Window::all().ok()?;
        for window in windows {
            let title = window.title().unwrap_or_default();
            if !crate::title::matches_title(&title, &self.title_hint) {
                continue;
            }
            if window.is_minimized().unwrap_or(false) {
                continue;
            }
            return Some(WindowInfo {
                title,
                x: window.x().unwrap_or(0),
                y: window.y().unwrap_or(0),
                width: window.width().unwrap_or(0),
                height: window.height().unwrap_or(0),
            });
        }
        None
    }
}