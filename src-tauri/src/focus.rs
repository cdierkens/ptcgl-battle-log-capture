//! Whether the overlays may show: only over the active game. Pure.

/// The process id of the active (frontmost) application.
pub trait FocusProbe {
    fn active_pid(&self) -> Option<u32>;
}

/// Remembers the last active app that wasn't us, so clicking into our own
/// overlay (to paste) keeps it visible when the player came from the game.
#[derive(Debug, Default)]
pub struct FocusTracker {
    last_external: Option<u32>,
}

impl FocusTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// An unknown game pid fails open: overlays show regardless of focus.
    pub fn observe(&mut self, active: Option<u32>, game_pid: Option<u32>, own_pid: u32) -> bool {
        if game_pid.is_none() {
            return true;
        }
        match active {
            Some(pid) if pid == own_pid => self.last_external.is_some() && self.last_external == game_pid,
            other => {
                self.last_external = other;
                other.is_some() && other == game_pid
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAME: u32 = 4242;
    const US: u32 = 7;
    const OTHER: u32 = 999;

    #[test]
    fn visible_while_the_game_is_active() {
        assert!(FocusTracker::new().observe(Some(GAME), Some(GAME), US));
    }

    #[test]
    fn stays_visible_when_we_become_active_from_the_game() {
        let mut f = FocusTracker::new();
        f.observe(Some(GAME), Some(GAME), US);
        assert!(f.observe(Some(US), Some(GAME), US));
    }

    #[test]
    fn hidden_when_we_become_active_from_another_app() {
        let mut f = FocusTracker::new();
        f.observe(Some(OTHER), Some(GAME), US);
        assert!(!f.observe(Some(US), Some(GAME), US));
    }

    #[test]
    fn hidden_while_another_app_is_active() {
        let mut f = FocusTracker::new();
        f.observe(Some(GAME), Some(GAME), US);
        assert!(!f.observe(Some(OTHER), Some(GAME), US));
        assert!(!f.observe(Some(US), Some(GAME), US));
    }

    #[test]
    fn visible_when_the_game_pid_is_unknown() {
        let mut f = FocusTracker::new();
        assert!(f.observe(Some(OTHER), None, US));
        assert!(f.observe(None, None, US));
    }

    #[test]
    fn hidden_when_the_active_app_is_unknown() {
        let mut f = FocusTracker::new();
        f.observe(Some(GAME), Some(GAME), US);
        assert!(!f.observe(None, Some(GAME), US));
        assert!(!f.observe(Some(US), Some(GAME), US));
    }
}
