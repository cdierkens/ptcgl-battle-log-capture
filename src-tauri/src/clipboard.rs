//! Opt-in clipboard auto-save: the probe port and a pure change watcher.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClipboardError {
    #[error("clipboard unavailable: {0}")]
    Unavailable(String),
    #[error("clipboard access is denied in System Settings → Privacy & Security")]
    Denied,
}

/// The OS's per-app clipboard permission (macOS 15.4+ `NSPasteboard.accessBehavior`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ClipboardAccess {
    /// Never prompted yet; the first programmatic read shows the system alert.
    Default,
    /// The system alerts on every programmatic read.
    Ask,
    Allowed,
    Denied,
}

pub trait ClipboardProbe {
    /// Cheap change counter; reading it never shows a privacy alert.
    fn change_count(&self) -> u64;
    fn access(&self) -> ClipboardAccess;
    /// `Ok(None)` when the content is concealed/transient (password managers) or not text.
    fn read_text(&self) -> Result<Option<String>, ClipboardError>;
}

/// Armed with a baseline count on entering the results screen; reports each later change once,
/// so a log already on the clipboard before the results screen is never picked up.
#[derive(Debug, Default)]
pub struct ClipboardWatch {
    baseline: Option<u64>,
}

impl ClipboardWatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn arm(&mut self, count: u64) {
        self.baseline = Some(count);
    }

    pub fn disarm(&mut self) {
        self.baseline = None;
    }

    pub fn is_armed(&self) -> bool {
        self.baseline.is_some()
    }

    /// True once per change since arming; the new count becomes the baseline.
    pub fn changed(&mut self, count: u64) -> bool {
        match self.baseline {
            Some(baseline) if baseline != count => {
                self.baseline = Some(count);
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unarmed_never_reports() {
        let mut w = ClipboardWatch::new();
        assert!(!w.changed(1));
        assert!(!w.is_armed());
    }

    #[test]
    fn reports_each_change_after_arming_once() {
        let mut w = ClipboardWatch::new();
        w.arm(5);
        assert!(!w.changed(5));
        assert!(w.changed(6));
        assert!(!w.changed(6));
        assert!(w.changed(7));
    }

    #[test]
    fn rearming_moves_the_baseline() {
        let mut w = ClipboardWatch::new();
        w.arm(5);
        w.arm(9);
        assert!(!w.changed(9));
    }

    #[test]
    fn disarm_stops_reporting() {
        let mut w = ClipboardWatch::new();
        w.arm(5);
        w.disarm();
        assert!(!w.changed(6));
    }

    #[test]
    fn access_serializes_for_the_ui() {
        assert_eq!(serde_json::to_value(ClipboardAccess::Denied).unwrap(), serde_json::json!("denied"));
    }
}
