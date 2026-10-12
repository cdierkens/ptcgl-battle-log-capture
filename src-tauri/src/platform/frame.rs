use crate::ports::{Frame, FrameCapturer, WindowInfo};

/// Captures the PTCG Live window via `xcap`.
pub struct XcapFrameCapturer;

impl FrameCapturer for XcapFrameCapturer {
    fn capture(&self, window: &WindowInfo) -> Option<Frame> {
        let windows = xcap::Window::all().ok()?;
        let target = windows.into_iter().find(|w| w.title().unwrap_or_default() == window.title)?;
        let img = target.capture_image().ok()?;
        Some(Frame { width: img.width(), height: img.height(), rgba: img.into_raw() })
    }
}
