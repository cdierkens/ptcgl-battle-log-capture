use crate::ports::ScreenCaptureAccess;

/// Screen-recording authorization. On macOS this delegates to CoreGraphics;
/// on every other platform it is always authorized (no such permission gate).
pub struct ScreenRecordingAccess;

#[cfg(target_os = "macos")]
impl ScreenCaptureAccess for ScreenRecordingAccess {
    fn is_authorized(&self) -> bool {
        objc2_core_graphics::CGPreflightScreenCaptureAccess()
    }

    fn request(&self) -> bool {
        objc2_core_graphics::CGRequestScreenCaptureAccess()
    }
}

#[cfg(not(target_os = "macos"))]
impl ScreenCaptureAccess for ScreenRecordingAccess {
    fn is_authorized(&self) -> bool {
        true
    }

    fn request(&self) -> bool {
        true
    }
}
