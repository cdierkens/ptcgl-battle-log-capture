/// Capture engine state machine. Pure — no side effects.
///
/// The in-game capture sequence is four steps: open the "Battle Log" tab, click
/// "Export", confirm the dialog, then dismiss it. Each `*Found` event is an OCR
/// detection and each `*Clicked` is the synthesized click that follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    Idle,
    PermissionRequired,
    Watching,
    LogTabDetected,
    PanelOpened,
    ExportDetected,
    ExportPressed,
    ConfirmDetected,
    ConfirmPressed,
    CloseDetected,
    Captured,
    /// A capture just completed. We stay here, idle, until the results screen
    /// is dismissed, so we never re-capture (or steal focus) while the user is
    /// still looking at the screen they just finished.
    AwaitingNextMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureEvent {
    WindowFound,
    WindowLost,
    PermissionDenied,
    LogTabFound,
    LogTabClicked,
    ExportFound,
    ExportClicked,
    ConfirmFound,
    ConfirmClicked,
    CloseFound,
    CloseClicked,
    LogCaptured,
    CooldownElapsed,
    /// The results screen is no longer showing; arm for the next match.
    ResultsScreenDismissed,
}

impl std::fmt::Display for CaptureState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Advance the capture state machine. Unknown (state, event) pairs are no-ops.
pub fn next(state: CaptureState, event: CaptureEvent) -> CaptureState {
    use CaptureEvent::*;
    use CaptureState::*;
    match (state, event) {
        (Idle, WindowFound) => Watching,
        (Idle, PermissionDenied) => PermissionRequired,
        (PermissionRequired, WindowFound) => Watching,
        (PermissionRequired, WindowLost) => Idle,
        (Watching, LogTabFound) => LogTabDetected,
        (Watching, WindowLost) => Idle,
        (LogTabDetected, LogTabClicked) => PanelOpened,
        (LogTabDetected, WindowLost) => Idle,
        (PanelOpened, ExportFound) => ExportDetected,
        (PanelOpened, WindowLost) => Idle,
        (ExportDetected, ExportClicked) => ExportPressed,
        (ExportDetected, WindowLost) => Idle,
        (ExportPressed, ConfirmFound) => ConfirmDetected,
        (ExportPressed, WindowLost) => Idle,
        (ConfirmDetected, ConfirmClicked) => ConfirmPressed,
        (ConfirmDetected, WindowLost) => Idle,
        (ConfirmPressed, CloseFound) => CloseDetected,
        (ConfirmPressed, WindowLost) => Idle,
        (CloseDetected, CloseClicked) => Captured,
        (CloseDetected, WindowLost) => Idle,
        (Captured, CooldownElapsed) => AwaitingNextMatch,
        (Captured, WindowLost) => Idle,
        (AwaitingNextMatch, ResultsScreenDismissed) => Watching,
        (AwaitingNextMatch, WindowLost) => Idle,
        _ => state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_to_watching_on_window_found() {
        assert_eq!(next(CaptureState::Idle, CaptureEvent::WindowFound), CaptureState::Watching);
    }

    #[test]
    fn idle_to_permission_required_on_denied() {
        assert_eq!(next(CaptureState::Idle, CaptureEvent::PermissionDenied), CaptureState::PermissionRequired);
    }

    #[test]
    fn permission_required_to_watching_on_window_found() {
        assert_eq!(next(CaptureState::PermissionRequired, CaptureEvent::WindowFound), CaptureState::Watching);
    }

    #[test]
    fn permission_required_to_idle_on_window_lost() {
        assert_eq!(next(CaptureState::PermissionRequired, CaptureEvent::WindowLost), CaptureState::Idle);
    }

    #[test]
    fn watching_to_log_tab_detected_on_log_tab_found() {
        assert_eq!(next(CaptureState::Watching, CaptureEvent::LogTabFound), CaptureState::LogTabDetected);
    }

    #[test]
    fn log_tab_detected_to_panel_opened_on_click() {
        assert_eq!(next(CaptureState::LogTabDetected, CaptureEvent::LogTabClicked), CaptureState::PanelOpened);
    }

    #[test]
    fn panel_opened_to_export_detected_on_export_found() {
        assert_eq!(next(CaptureState::PanelOpened, CaptureEvent::ExportFound), CaptureState::ExportDetected);
    }

    #[test]
    fn export_detected_to_export_pressed_on_click() {
        assert_eq!(next(CaptureState::ExportDetected, CaptureEvent::ExportClicked), CaptureState::ExportPressed);
    }

    #[test]
    fn export_pressed_to_confirm_detected_on_confirm_found() {
        assert_eq!(next(CaptureState::ExportPressed, CaptureEvent::ConfirmFound), CaptureState::ConfirmDetected);
    }

    #[test]
    fn confirm_detected_to_confirm_pressed_on_click() {
        assert_eq!(next(CaptureState::ConfirmDetected, CaptureEvent::ConfirmClicked), CaptureState::ConfirmPressed);
    }

    #[test]
    fn confirm_pressed_to_close_detected_on_close_found() {
        assert_eq!(next(CaptureState::ConfirmPressed, CaptureEvent::CloseFound), CaptureState::CloseDetected);
    }

    #[test]
    fn close_detected_to_captured_on_click() {
        assert_eq!(next(CaptureState::CloseDetected, CaptureEvent::CloseClicked), CaptureState::Captured);
    }

    #[test]
    fn captured_waits_for_results_screen_to_clear_before_rearming() {
        assert_eq!(next(CaptureState::Captured, CaptureEvent::CooldownElapsed), CaptureState::AwaitingNextMatch);
        assert_eq!(next(CaptureState::AwaitingNextMatch, CaptureEvent::ResultsScreenDismissed), CaptureState::Watching);
    }

    #[test]
    fn awaiting_next_match_ignores_repeated_log_tab_detections() {
        // While the user is still on the results screen we must not re-arm, or
        // we would re-open the drawer and re-capture the same log.
        assert_eq!(next(CaptureState::AwaitingNextMatch, CaptureEvent::LogTabFound), CaptureState::AwaitingNextMatch);
        assert_eq!(next(CaptureState::AwaitingNextMatch, CaptureEvent::LogTabClicked), CaptureState::AwaitingNextMatch);
    }

    #[test]
    fn window_lost_returns_to_idle_from_any_active_state() {
        for state in [
            CaptureState::Watching,
            CaptureState::LogTabDetected,
            CaptureState::PanelOpened,
            CaptureState::ExportDetected,
            CaptureState::ExportPressed,
            CaptureState::ConfirmDetected,
            CaptureState::ConfirmPressed,
            CaptureState::CloseDetected,
            CaptureState::Captured,
            CaptureState::AwaitingNextMatch,
        ] {
            assert_eq!(next(state, CaptureEvent::WindowLost), CaptureState::Idle, "from {state}");
        }
    }

    #[test]
    fn unknown_pairs_are_noops() {
        assert_eq!(next(CaptureState::Idle, CaptureEvent::LogCaptured), CaptureState::Idle);
        assert_eq!(next(CaptureState::Captured, CaptureEvent::LogTabFound), CaptureState::Captured);
        // Out-of-order detections must not advance the machine.
        assert_eq!(next(CaptureState::Watching, CaptureEvent::ExportFound), CaptureState::Watching);
        assert_eq!(next(CaptureState::PanelOpened, CaptureEvent::ConfirmFound), CaptureState::PanelOpened);
    }
}
