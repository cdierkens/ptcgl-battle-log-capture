//! The general pasteboard, via AppKit.

use crate::clipboard::{ClipboardAccess, ClipboardError, ClipboardProbe};

pub struct SystemClipboard;

/// Password managers mark secrets with these (nspasteboard.org); never read them.
#[cfg(target_os = "macos")]
const SKIPPED_TYPES: [&str; 2] = ["org.nspasteboard.ConcealedType", "org.nspasteboard.TransientType"];

#[cfg(target_os = "macos")]
impl ClipboardProbe for SystemClipboard {
    fn change_count(&self) -> u64 {
        objc2_app_kit::NSPasteboard::generalPasteboard().changeCount() as u64
    }

    fn access(&self) -> ClipboardAccess {
        use objc2::runtime::NSObjectProtocol;
        use objc2::sel;
        use objc2_app_kit::{NSPasteboard, NSPasteboardAccessBehavior};

        let pasteboard = NSPasteboard::generalPasteboard();
        // `accessBehavior` is macOS 15.4+; older systems never alert.
        if !pasteboard.respondsToSelector(sel!(accessBehavior)) {
            return ClipboardAccess::Allowed;
        }
        match pasteboard.accessBehavior() {
            NSPasteboardAccessBehavior::Ask => ClipboardAccess::Ask,
            NSPasteboardAccessBehavior::AlwaysAllow => ClipboardAccess::Allowed,
            NSPasteboardAccessBehavior::AlwaysDeny => ClipboardAccess::Denied,
            _ => ClipboardAccess::Default,
        }
    }

    fn read_text(&self) -> Result<Option<String>, ClipboardError> {
        use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};

        if self.access() == ClipboardAccess::Denied {
            return Err(ClipboardError::Denied);
        }
        let pasteboard = NSPasteboard::generalPasteboard();
        let concealed = pasteboard
            .types()
            .is_some_and(|types| types.iter().any(|t| SKIPPED_TYPES.contains(&t.to_string().as_str())));
        if concealed {
            return Ok(None);
        }
        // SAFETY: an immutable AppKit constant.
        let string_type = unsafe { NSPasteboardTypeString };
        Ok(pasteboard.stringForType(string_type).map(|s| s.to_string()))
    }
}

#[cfg(not(target_os = "macos"))]
impl ClipboardProbe for SystemClipboard {
    fn change_count(&self) -> u64 {
        0
    }

    fn access(&self) -> ClipboardAccess {
        ClipboardAccess::Allowed
    }

    fn read_text(&self) -> Result<Option<String>, ClipboardError> {
        Ok(None)
    }
}
