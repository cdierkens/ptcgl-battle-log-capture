use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};

use crate::ports::{InputController, WindowActivator};

/// Brings the PTCG Live process to the foreground via AppleScript so a
/// synthesized click is delivered to the game rather than whatever app happens
/// to be frontmost (the capture app runs in the background, so the terminal is
/// usually frontmost and would otherwise swallow the click).
///
/// This uses `osascript` (~200ms) rather than native `NSRunningApplication`
/// activation. Native activation is only fast when it can dispatch to a live
/// AppKit main run loop; called from the engine's background thread in
/// isolation it stalled (~1s). `osascript` is a separate process, so it is
/// robust to the app's main-thread state and keeps each click predictable.
#[cfg(target_os = "macos")]
pub struct MacWindowActivator;

#[cfg(target_os = "macos")]
impl WindowActivator for MacWindowActivator {
    fn activate_game(&self, process_name: &str) -> Result<(), String> {
        let script = format!(
            "tell application \"System Events\" to set frontmost of (first process whose name is \"{}\") to true",
            process_name.replace('\\', "\\\\").replace('"', "\\\"")
        );
        let status = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("osascript exited with {status}"))
        }
    }
}

/// Non-macOS: no window activation is needed (clicks are delivered to the
/// foreground window differently), so this is a no-op.
#[cfg(not(target_os = "macos"))]
pub struct MacWindowActivator;

#[cfg(not(target_os = "macos"))]
impl WindowActivator for MacWindowActivator {
    fn activate_game(&self, _process_name: &str) -> Result<(), String> {
        Ok(())
    }
}

/// Simulates mouse clicks via `enigo`.
pub struct EnigoInput;

impl InputController for EnigoInput {
    fn click(&self, x: i32, y: i32) -> Result<(), String> {
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        enigo.move_mouse(x, y, Coordinate::Abs).map_err(|e| e.to_string())?;
        enigo.button(Button::Left, Direction::Click).map_err(|e| e.to_string())?;
        Ok(())
    }
}