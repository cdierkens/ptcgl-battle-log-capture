//! Overlay state machine. Pure — no side effects.

use crate::overlay_geometry::ScreenRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShieldStatus {
    Up,
    Saved,
    Dismissed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverlayState {
    PermissionRequired,
    NoWindow,
    InMatch,
    /// `continue_rect` is window-local, so the shield follows window moves.
    Results { shield: ShieldStatus, continue_rect: Option<ScreenRect> },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverlayEvent {
    PermissionDenied,
    WindowFound,
    WindowLost,
    ResultsDetected,
    ContinueLocated(ScreenRect),
    ResultsGone,
    LogSaved,
    /// The auto-saved log was undone (moved to the Trash).
    SaveUndone,
    Dismissed,
}

/// Advance the overlay state machine. Unlisted (state, event) pairs are no-ops.
pub fn transition(state: OverlayState, event: OverlayEvent) -> OverlayState {
    use OverlayEvent as E;
    use OverlayState as S;
    match (state, event) {
        (_, E::PermissionDenied) => S::PermissionRequired,
        (S::PermissionRequired | S::NoWindow, E::WindowFound) => S::InMatch,
        (S::PermissionRequired | S::InMatch | S::Results { .. }, E::WindowLost) => S::NoWindow,
        (S::InMatch, E::ResultsDetected) => S::Results { shield: ShieldStatus::Up, continue_rect: None },
        (S::Results { shield, .. }, E::ContinueLocated(rect)) => S::Results { shield, continue_rect: Some(rect) },
        (S::Results { shield: ShieldStatus::Up | ShieldStatus::Dismissed, continue_rect }, E::LogSaved) => {
            S::Results { shield: ShieldStatus::Saved, continue_rect }
        }
        (S::Results { shield: ShieldStatus::Saved, continue_rect }, E::SaveUndone) => {
            S::Results { shield: ShieldStatus::Up, continue_rect }
        }
        (S::Results { shield: ShieldStatus::Up, continue_rect }, E::Dismissed) => {
            S::Results { shield: ShieldStatus::Dismissed, continue_rect }
        }
        (S::Results { .. }, E::ResultsGone) => S::InMatch,
        (s, _) => s,
    }
}

impl OverlayState {
    /// Whether the capture drawer (handle or open drawer) shows over the game.
    pub fn drawer_visible(&self) -> bool {
        matches!(self, OverlayState::InMatch | OverlayState::Results { .. })
    }

    /// The window-local shield rect, or `None` when the shield is hidden or prompts are off.
    pub fn shield_rect(&self, prompts: bool) -> Option<ScreenRect> {
        match self {
            OverlayState::Results { shield: ShieldStatus::Up, continue_rect: Some(rect) } if prompts => Some(*rect),
            _ => None,
        }
    }

    /// Stable name shared with the UI (`capture-status`, `overlay://state`, `get_capture_state`).
    pub fn name(&self) -> &'static str {
        match self {
            OverlayState::PermissionRequired => "PermissionRequired",
            OverlayState::NoWindow => "NoWindow",
            OverlayState::InMatch => "InMatch",
            OverlayState::Results { shield: ShieldStatus::Up, .. } => "Results",
            OverlayState::Results { shield: ShieldStatus::Saved, .. } => "Saved",
            OverlayState::Results { shield: ShieldStatus::Dismissed, .. } => "Dismissed",
        }
    }
}

impl std::fmt::Display for OverlayState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use OverlayEvent as E;
    use OverlayState as S;

    const RECT: ScreenRect = ScreenRect { x: 1.0, y: 2.0, width: 240.0, height: 44.0 };
    const UP: S = S::Results { shield: ShieldStatus::Up, continue_rect: None };
    const UP_LOCATED: S = S::Results { shield: ShieldStatus::Up, continue_rect: Some(RECT) };

    #[test]
    fn permission_denied_from_any_state() {
        for s in [S::NoWindow, S::InMatch, UP_LOCATED] {
            assert_eq!(transition(s, E::PermissionDenied), S::PermissionRequired);
        }
    }

    #[test]
    fn window_found_enters_the_match() {
        assert_eq!(transition(S::NoWindow, E::WindowFound), S::InMatch);
        assert_eq!(transition(S::PermissionRequired, E::WindowFound), S::InMatch);
        assert_eq!(transition(UP_LOCATED, E::WindowFound), UP_LOCATED);
    }

    #[test]
    fn window_lost_returns_to_no_window() {
        for s in [S::PermissionRequired, S::InMatch, UP_LOCATED] {
            assert_eq!(transition(s, E::WindowLost), S::NoWindow);
        }
    }

    #[test]
    fn results_raise_the_shield_then_locate_continue() {
        let s = transition(S::InMatch, E::ResultsDetected);
        assert_eq!(s, UP);
        assert_eq!(transition(s, E::ContinueLocated(RECT)), UP_LOCATED);
    }

    #[test]
    fn saving_or_dismissing_lowers_the_shield() {
        let saved = transition(UP_LOCATED, E::LogSaved);
        assert_eq!(saved, S::Results { shield: ShieldStatus::Saved, continue_rect: Some(RECT) });
        let dismissed = transition(UP_LOCATED, E::Dismissed);
        assert_eq!(dismissed, S::Results { shield: ShieldStatus::Dismissed, continue_rect: Some(RECT) });
        assert_eq!(transition(dismissed, E::LogSaved), saved);
        assert_eq!(transition(saved, E::Dismissed), saved);
    }

    #[test]
    fn undoing_a_save_raises_the_shield_again() {
        let saved = transition(UP_LOCATED, E::LogSaved);
        assert_eq!(transition(saved, E::SaveUndone), UP_LOCATED);
        assert_eq!(transition(UP_LOCATED, E::SaveUndone), UP_LOCATED);
        assert_eq!(transition(S::InMatch, E::SaveUndone), S::InMatch);
    }

    #[test]
    fn prompts_off_hides_the_shield() {
        assert_eq!(UP_LOCATED.shield_rect(true), Some(RECT));
        assert_eq!(UP_LOCATED.shield_rect(false), None);
    }

    #[test]
    fn results_gone_returns_to_the_match() {
        assert_eq!(transition(UP_LOCATED, E::ResultsGone), S::InMatch);
    }

    #[test]
    fn saving_mid_match_changes_nothing() {
        assert_eq!(transition(S::InMatch, E::LogSaved), S::InMatch);
        assert_eq!(transition(S::InMatch, E::Dismissed), S::InMatch);
    }

    #[test]
    fn drawer_and_shield_follow_the_state() {
        assert!(!S::PermissionRequired.drawer_visible());
        assert!(!S::NoWindow.drawer_visible());
        assert!(S::InMatch.drawer_visible());
        assert!(UP.drawer_visible());
        assert_eq!(UP.shield_rect(true), None);
        assert_eq!(UP_LOCATED.shield_rect(true), Some(RECT));
        let saved = transition(UP_LOCATED, E::LogSaved);
        assert!(saved.drawer_visible());
        assert_eq!(saved.shield_rect(true), None);
        assert_eq!(transition(UP_LOCATED, E::Dismissed).shield_rect(true), None);
    }

    #[test]
    fn names_are_stable() {
        assert_eq!(S::InMatch.to_string(), "InMatch");
        assert_eq!(UP.name(), "Results");
        assert_eq!(transition(UP, E::LogSaved).name(), "Saved");
        assert_eq!(transition(UP, E::Dismissed).name(), "Dismissed");
    }
}
