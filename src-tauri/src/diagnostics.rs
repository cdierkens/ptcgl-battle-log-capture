//! The system half of "Copy diagnostics"; the Help tab appends its event history.

use crate::clipboard::ClipboardAccess;

/// No paths or player name: diagnostics get pasted into public bug reports.
#[derive(Debug, Clone)]
pub struct Diagnostics {
    pub version: String,
    pub os: String,
    pub state: String,
    pub screen_recording: bool,
    pub clipboard_access: ClipboardAccess,
    pub prompt_after_match: bool,
    pub clipboard_autosave: bool,
    pub saved_logs: Result<usize, String>,
}

impl Diagnostics {
    pub fn report(&self) -> String {
        let on = |b: bool| if b { "on" } else { "off" };
        let saved_logs = match &self.saved_logs {
            Ok(n) => n.to_string(),
            Err(e) => format!("unavailable ({e})"),
        };
        [
            format!("Battle Log Capture {}", self.version),
            format!("OS: {}", self.os),
            format!("State: {}", self.state),
            format!("Screen Recording: {}", if self.screen_recording { "granted" } else { "not granted" }),
            format!("Clipboard access: {}", clipboard_label(self.clipboard_access)),
            format!("Prompt to save after each match: {}", on(self.prompt_after_match)),
            format!("Clipboard auto-save: {}", on(self.clipboard_autosave)),
            format!("Saved logs: {saved_logs}"),
        ]
        .join("\n")
    }
}

fn clipboard_label(access: ClipboardAccess) -> &'static str {
    match access {
        ClipboardAccess::Default => "not asked yet",
        ClipboardAccess::Ask => "ask each time",
        ClipboardAccess::Allowed => "allowed",
        ClipboardAccess::Denied => "denied",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_lists_every_field() {
        let d = Diagnostics {
            version: "0.1.0".into(),
            os: "macos aarch64".into(),
            state: "Results".into(),
            screen_recording: true,
            clipboard_access: ClipboardAccess::Ask,
            prompt_after_match: true,
            clipboard_autosave: false,
            saved_logs: Ok(12),
        };
        assert_eq!(
            d.report(),
            "Battle Log Capture 0.1.0\nOS: macos aarch64\nState: Results\nScreen Recording: granted\n\
             Clipboard access: ask each time\nPrompt to save after each match: on\n\
             Clipboard auto-save: off\nSaved logs: 12"
        );
    }

    #[test]
    fn an_unreadable_folder_is_reported_not_fatal() {
        let d = Diagnostics {
            version: "0.1.0".into(),
            os: "macos".into(),
            state: "NoWindow".into(),
            screen_recording: false,
            clipboard_access: ClipboardAccess::Default,
            prompt_after_match: false,
            clipboard_autosave: false,
            saved_logs: Err("permission denied".into()),
        };
        assert!(d.report().ends_with("Saved logs: unavailable (permission denied)"));
    }
}
