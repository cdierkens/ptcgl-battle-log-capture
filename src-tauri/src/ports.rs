/// Platform side-effect ports. The pure core logic depends only on these trait
/// signatures; concrete implementations (xcap window lookup and capture, Vision
/// OCR, filesystem save, Tauri overlay windows) are wired behind these so the
/// core stays unit-testable without a live game.
use crate::overlay_geometry::ScreenRect;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub title: String,
    /// The owning process, used to tell whether the game is the active app.
    /// `None` when the OS didn't report it.
    pub pid: Option<u32>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// A captured frame: raw RGBA bytes plus dimensions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub trait WindowProvider {
    fn find_game_window(&self) -> Option<WindowInfo>;
}

pub trait FrameCapturer {
    fn capture(&self, window: &WindowInfo) -> Option<Frame>;
}

pub trait LogStore {
    /// Save the battle-log text, returning the written file path on success.
    fn save(&self, text: &str) -> Result<String, String>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OverlayError {
    #[error("overlay window `{label}` is unavailable")]
    WindowUnavailable { label: &'static str },
    #[error("overlay window `{label}` failed: {reason}")]
    Window { label: &'static str, reason: String },
}

/// Shows, moves, and hides the capture drawer and Continue shield windows.
pub trait OverlayPresenter: Send + Sync {
    fn show_drawer(&self, game: &WindowInfo) -> Result<(), OverlayError>;
    /// Resize the drawer window for the handle (closed) or the drawer (open).
    fn set_drawer_open(&self, open: bool) -> Result<(), OverlayError>;
    fn hide_drawer(&self) -> Result<(), OverlayError>;
    /// `local` is window-local; the presenter offsets it by the game origin.
    fn show_shield(&self, game: &WindowInfo, local: ScreenRect) -> Result<(), OverlayError>;
    fn hide_shield(&self) -> Result<(), OverlayError>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecretError {
    #[error("this platform has no secret store")]
    Unsupported,
    #[error("secret store failed: {0}")]
    Store(String),
}

/// OS keychain for destination tokens, keyed by an app-chosen account name.
pub trait SecretStore: Send + Sync {
    fn get(&self, account: &str) -> Result<Option<String>, SecretError>;
    fn set(&self, account: &str, secret: &str) -> Result<(), SecretError>;
    /// Deleting a missing entry is not an error.
    fn delete(&self, account: &str) -> Result<(), SecretError>;
}

/// Screen-recording authorization (macOS). On other platforms this is always
/// authorized, so the concrete implementation reports `true`.
pub trait ScreenCaptureAccess {
    /// True if the OS has granted screen-recording access.
    fn is_authorized(&self) -> bool;
    /// Request authorization, triggering the OS prompt if undetermined.
    /// Returns true if authorized after the call.
    fn request(&self) -> bool;
}

/// A recognized text region within a captured frame, in frame-local pixel
/// coordinates (top-left origin).
#[derive(Debug, Clone, PartialEq)]
pub struct TextMatch {
    pub text: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub trait TextRecognizer {
    /// Recognize text in `frame` and return the matches whose text equals one
    /// of `needles` (case-insensitive, trimmed). Coordinates are frame-local.
    fn find_text(&self, frame: &Frame, needles: &[String]) -> Vec<TextMatch>;

    /// Like `find_text`, but a match is any recognized string that *contains*
    /// one of `needles` as a substring.
    fn find_text_containing(&self, frame: &Frame, needles: &[String]) -> Vec<TextMatch>;

    /// Pay any one-time startup cost. Blocks, so run it off the engine thread.
    fn warm_up(&self) {}
}
