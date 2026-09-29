/// Platform side-effect ports. The pure core logic depends only on these trait
/// signatures; concrete implementations (xcap capture, enigo clicks, arboard
/// clipboard, filesystem save) are wired behind these so the core stays
/// unit-testable without a live game.
use image::RgbaImage;

use crate::coords::{Point, RelativeRect};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub title: String,
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

pub trait InputController {
    fn click(&self, x: i32, y: i32) -> Result<(), String>;
}

/// Brings the game process to the foreground so a synthesized click is
/// delivered to it rather than to whatever app is currently frontmost.
pub trait WindowActivator {
    fn activate_game(&self, process_name: &str) -> Result<(), String>;
}

pub trait ClipboardReader {
    fn read_text(&self) -> Option<String>;
}

pub trait LogStore {
    /// Save the battle-log text, returning the written file path on success.
    fn save(&self, text: &str) -> Result<String, String>;
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
    /// one of `needles` as a substring. Vision frequently merges a small toast
    /// with neighbouring text (e.g. "Export Copied!" + adjacent label), so this
    /// is used for the transient confirmation toast.
    fn find_text_containing(&self, frame: &Frame, needles: &[String]) -> Vec<TextMatch>;
}

/// Finds a calibrated reference template within a captured frame, returning the
/// template center in frame pixel coordinates (top-left origin). `template_scale`
/// and `frame_scale` are the pixel densities (frame px / window pt) of the
/// template and frame respectively, so the matcher can rescale the template to
/// the frame's density before matching. `search` optionally confines the search
/// to a window-fraction region (the return point is still full-frame).
pub trait TemplateMatcher {
    fn find(
        &self,
        frame: &Frame,
        template: &RgbaImage,
        template_scale: f32,
        frame_scale: f32,
        search: Option<RelativeRect>,
    ) -> Option<Point>;
}