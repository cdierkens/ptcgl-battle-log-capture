//! The active application's process id, via AppKit.

use crate::focus::FocusProbe;

pub struct WorkspaceFocusProbe;

#[cfg(target_os = "macos")]
impl FocusProbe for WorkspaceFocusProbe {
    fn active_pid(&self) -> Option<u32> {
        use objc2_app_kit::NSWorkspace;
        use objc2_foundation::{NSNumber, NSString};

        let pid_key = NSString::from_str("NSApplicationProcessIdentifier");
        // `frontmostApplication` goes stale off the main run loop (this runs on the
        // engine thread); the deprecated `activeApplication` is real-time.
        #[allow(deprecated)]
        let active = NSWorkspace::sharedWorkspace().activeApplication();
        active
            .and_then(|dict| dict.valueForKey(&pid_key))
            .and_then(|pid| pid.downcast::<NSNumber>().ok())
            .map(|pid| pid.intValue() as u32)
    }
}

#[cfg(not(target_os = "macos"))]
impl FocusProbe for WorkspaceFocusProbe {
    fn active_pid(&self) -> Option<u32> {
        None
    }
}
