use crate::ports::ClipboardReader;

/// Reads the system clipboard via `arboard`.
pub struct ArboardClipboard;

impl ClipboardReader for ArboardClipboard {
    fn read_text(&self) -> Option<String> {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        clipboard.get_text().ok()
    }
}

impl ArboardClipboard {
    /// Replace the clipboard with an empty string.
    ///
    /// Called immediately before clicking Export so that whatever the Export
    /// writes is unambiguously *this* match's log. Without this, a leftover log
    /// from a previous match can sit on the clipboard; if the Export click fails
    /// to copy, the engine would read that stale log, see it as "already
    /// captured", and falsely report success while saving nothing.
    pub fn clear(&self) -> bool {
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => clipboard.set_text("").is_ok(),
            Err(_) => false,
        }
    }
}
