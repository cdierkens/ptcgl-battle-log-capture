use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tracing::{debug, error, info, warn};

use crate::buttons::{
    battle_log_needles, battle_log_region, close_icon, confirm_needles, drawer_title_region,
    export_copied_needles, export_icon, RelativePoint,
};
use crate::coords::{crop_frame, frame_to_point_scale, rect_center, window_to_screen, Point};
use crate::dedupe::SeenHashes;
use crate::events::CaptureStatus;
use crate::gate::looks_like_battle_log;
use crate::platform::clipboard::ArboardClipboard;
use crate::platform::frame::XcapFrameCapturer;
use crate::platform::input::{EnigoInput, MacWindowActivator};
use crate::platform::ocr::VisionTextRecognizer;
use crate::platform::permission::ScreenRecordingAccess;
use crate::platform::store::FsLogStore;
use crate::platform::window::XcapWindowProvider;
use crate::ports::{
    ClipboardReader, Frame, FrameCapturer, InputController, LogStore, ScreenCaptureAccess, TextMatch,
    TextRecognizer, WindowActivator, WindowInfo, WindowProvider,
};
use crate::settings;
use crate::state::{next, CaptureEvent, CaptureState};

/// How long to wait for the next UI element to appear after a click before
/// giving up on the current step and rescanning.
const STEP_TIMEOUT_SECS: u64 = 8;
/// Poll interval between frames. OCR is heavier than template matching, so a
/// sub-second cadence is enough to feel instant.
const POLL_MS: u64 = 600;
/// Faster poll used while watching and while a capture step is transient: the
/// short-lived "Export Copied!" toast lives ~400-600ms, and while watching we
/// only OCR a small region (~60ms) so a tight cadence is still cheap. This is
/// what lets us catch the results screen before the player reaches Continue.
const POLL_FAST_MS: u64 = 120;
/// Every Nth watch poll we sweep the whole frame instead of the small region, as
/// a safety net in case the Battle Log tab ever renders outside its measured
/// region (different locale layout, resized window).
const FULL_FRAME_SWEEP_EVERY: u32 = 15;
/// Tauri event emitted on every state transition.
const CAPTURE_STATUS_EVENT: &str = "capture-status";
/// macOS process name of the game window, used to focus it before clicking.
const GAME_PROCESS_NAME: &str = "Pokemon TCG Live";
/// Settle time after raising the game window, before the synthesized click.
const FOCUS_SETTLE_MS: u64 = 250;
/// If the Battle Log drawer hasn't shown its title this soon after a tab
/// click, re-click the tab (the first click often lands before the drawer is
/// interactive).
const DRAWER_RETRY_MS: u64 = 450;
/// Give up waiting for the drawer to open after this many re-clicks and click
/// Export anyway (best effort), so the sequence always makes progress.
const DRAWER_MAX_RECLICKS: u32 = 5;
/// Time to let the confirm dialog settle before dismissing it.
const DIALOG_SETTLE_MS: u64 = 500;
/// Grace period after a capture before we start looking for the results screen
/// to clear, so the drawer dismissal animation is not read as "screen gone".
const REARM_DELAY_MS: u64 = 1500;
/// Consecutive polls that must show no Battle Log tab before re-arming.
const REARM_CLEAR_POLLS: u32 = 3;

/// Background capture loop. Two-stage OCR flow: recognize the localized
/// "Battle Log" control on the results screen and click it to open the log
/// panel, then recognize the "Export" button in the panel and click it to copy
/// the log to the clipboard.
pub fn run(app: AppHandle, running: Arc<AtomicBool>, state_cell: Arc<Mutex<CaptureState>>) {
    let settings = settings::load();
    let output_dir = PathBuf::from(&settings.output_dir);

    let window_provider = XcapWindowProvider::new(crate::title::GAME_TITLE_HINT);
    let frame_capturer = XcapFrameCapturer;
    let text_recognizer = VisionTextRecognizer;
    let input = EnigoInput;
    let activator = MacWindowActivator;
    let clipboard = ArboardClipboard;
    let store = FsLogStore::new(output_dir.clone());
    let mut seen = SeenHashes::new();

    let screen_access = ScreenRecordingAccess;
    let mut permission_requested = false;

    let mut state = CaptureState::Idle;
    let mut step_at: Option<Instant> = None;
    let mut results_clear_polls: u32 = 0;
    let mut watch_polls: u32 = 0;
    // Screen coords of the Battle Log tab click, kept so we can re-click it
    // while waiting for the drawer to open.
    let mut tab_click: Option<(i32, i32)> = None;
    let mut tab_reclicks: u32 = 0;

    let mut was_running = running.load(Ordering::Relaxed);

    loop {
        // Publish the current state every iteration so the UI can always fetch
        // the latest capture state on mount, even if it missed the boot-time
        // event. Cheap write; does not affect capture timing.
        if let Ok(mut cell) = state_cell.lock() {
            *cell = state;
        }

        // Start/stop gate. When stopped, idle: no window lookup, no capture,
        // no focus activation, no clipboard poll. Single long-lived thread, so
        // the timing-sensitive capture path is untouched when running.
        let is_running = running.load(Ordering::Relaxed);
        if !is_running {
            if was_running {
                emit_status(&app, "Stopped", "Capture stopped", None);
                was_running = false;
            }
            std::thread::sleep(Duration::from_millis(250));
            continue;
        }
        if !was_running {
            emit_status(&app, "Running", "Capture started", None);
            was_running = true;
        }

        if !screen_access.is_authorized() {
            if !permission_requested {
                screen_access.request();
                permission_requested = true;
            }
            advance(&app, &mut state, CaptureEvent::PermissionDenied, "Screen Recording permission required", Some("grant access in System Settings > Privacy & Security > Screen Recording".to_string()));
            std::thread::sleep(Duration::from_secs(5));
            continue;
        }
        permission_requested = false;

        // Passive safety net: if a battle log is sitting on the clipboard — the
        // user clicked Export manually, or the OCR click flow missed — capture
        // it here regardless of state. Hash dedupe makes repeats no-ops.
        capture_clipboard(&clipboard, &store, &mut seen);

        let Some(window) = window_provider.find_game_window() else {
            advance(&app, &mut state, CaptureEvent::WindowLost, "Waiting for game window", None);
            std::thread::sleep(Duration::from_secs(1));
            continue;
        };

        advance(&app, &mut state, CaptureEvent::WindowFound, "Game window found", Some(window.title.clone()));

        let Some(frame) = frame_capturer.capture(&window) else {
            std::thread::sleep(Duration::from_secs(1));
            continue;
        };

        match state {
            CaptureState::Watching => {
                // The Battle Log tab is the earliest reliable signal that a
                // match has ended — every other result string (the reason
                // lines, the victory/defeat banner) renders on the same results
                // screen, so there is no usable pre-results signal to arm on.
                //
                // The tab renders at a fixed spot on the results screen, so we
                // OCR only that small region (~60ms) instead of the whole frame
                // (~260ms) and can poll fast enough to beat the player's
                // Continue click. Every FULL_FRAME_SWEEP_EVERY polls we fall
                // back to a full-frame sweep, so a layout change can never make
                // us miss the tab permanently. The sweep filters matches with
                // is_results_screen_tab so a "BATTLE LOG" hit from the *drawer's*
                // own tab (top of the screen) can't trigger a capture.
                watch_polls = watch_polls.wrapping_add(1);
                let needles = battle_log_needles();
                let full_sweep = watch_polls % FULL_FRAME_SWEEP_EVERY == 0;
                let mut hit: Option<TextMatch> = None;
                if full_sweep {
                    hit = text_recognizer
                        .find_text(&frame, &needles)
                        .into_iter()
                        .find(|m| is_results_screen_tab(m, &frame));
                } else if let Some(crop) = crop_frame(&frame, battle_log_region()) {
                    if let Some(m) = text_recognizer.find_text(&crop.frame, &needles).into_iter().next() {
                        // Map the region-local match back to full-frame pixels.
                        hit = Some(TextMatch {
                            x: m.x + crop.origin_x,
                            y: m.y + crop.origin_y,
                            ..m
                        });
                    }
                }
                if let Some(m) = hit {
                    log_match("Battle Log", &m, &window, &frame);
                    let detail = format!("{} at ({},{}) {}x{}", m.text, m.x, m.y, m.width, m.height);
                    advance(&app, &mut state, CaptureEvent::LogTabFound, "Results screen detected", Some(detail));
                    let (cx, cy) = click_point(&window, &frame, &m);
                    tab_click = Some((cx, cy));
                    tab_reclicks = 0;
                    if click_at(&activator, &input, cx, cy).is_ok() {
                        advance(&app, &mut state, CaptureEvent::LogTabClicked, "Battle Log clicked", Some(format!("({}, {})", cx, cy)));
                        step_at = Some(Instant::now());
                    }
                }
            }
            CaptureState::PanelOpened => {
                // Wait for the drawer to actually open instead of a fixed delay.
                // Opening the drawer reveals its own "Battle Log" title at the
                // top of the window (a different label from the results-screen
                // tab at the bottom, which is what we clicked). If that title
                // hasn't appeared yet, re-click the tab: the first click often
                // lands before the drawer is interactive, which is why the
                // export used to need a second full attempt.
                let open = crop_frame(&frame, drawer_title_region())
                    .map(|c| !text_recognizer.find_text(&c.frame, &battle_log_needles()).is_empty())
                    .unwrap_or(false);
                let out_of_retries = tab_reclicks >= DRAWER_MAX_RECLICKS;
                if open || out_of_retries {
                    if !open {
                        warn!(reclicks = tab_reclicks, "drawer never confirmed open; clicking Export anyway");
                    } else {
                        info!("Battle Log drawer open (title detected)");
                    }
                    // Clear the clipboard *before* Export so that whatever
                    // lands on it afterwards is unambiguously this match's
                    // log. Otherwise a leftover log from a previous match can
                    // sit on the clipboard, and if the Export click fails to
                    // copy, the engine reads that stale log, sees it as
                    // "already captured", and falsely reports success while
                    // saving nothing.
                    if clipboard.clear() {
                        info!("cleared clipboard before Export");
                    } else {
                        warn!("failed to clear clipboard before Export");
                    }
                    let (cx, cy) = relative_click(&window, &export_icon());
                    info!(label = "Export icon", click_x = cx, click_y = cy, "clicking fixed-position Export icon");
                    advance(&app, &mut state, CaptureEvent::ExportFound, "Export icon targeted", Some(format!("({}, {})", cx, cy)));
                    tab_click = None;
                    if click_at(&activator, &input, cx, cy).is_ok() {
                        advance(&app, &mut state, CaptureEvent::ExportClicked, "Export clicked", Some(format!("({}, {})", cx, cy)));
                        step_at = Some(Instant::now());
                    }
                } else if let (Some(t), Some((tx, ty))) = (step_at, tab_click) {
                    // Only re-click when the drawer is *believed closed* (its
                    // title is absent). The Battle Log tab is a toggle — a
                    // blind re-click would close an already-open drawer — so we
                    // gate every tab click on the drawer being closed.
                    if t.elapsed() >= Duration::from_millis(DRAWER_RETRY_MS) {
                        tab_reclicks += 1;
                        info!(attempt = tab_reclicks, "drawer closed; clicking Battle Log tab to open it");
                        let _ = click_at(&activator, &input, tx, ty);
                        step_at = Some(Instant::now());
                    }
                }
            }
            CaptureState::ExportPressed => {
                // The clipboard is ground truth and updates the instant Export
                // is clicked, so read it aggressively (~120ms) rather than
                // waiting for the short-lived "Export Copied!" toast, which is
                // easy to miss and previously cost ~18s. A log that is already
                // saved (the passive poll may have grabbed it) counts as
                // success too. The toast and a possible Confirm dialog are
                // handled as secondary signals.
                match capture_clipboard(&clipboard, &store, &mut seen) {
                    outcome @ (ClipboardCapture::Saved(_) | ClipboardCapture::AlreadyCaptured) => {
                        if let ClipboardCapture::Saved(path) = &outcome {
                            advance(&app, &mut state, CaptureEvent::ConfirmFound, "Battle log on clipboard", Some(path.clone()));
                        }
                        finish_capture(&app, &activator, &input, &window, &clipboard, &store, &mut seen, &mut state, &mut step_at);
                    }
                    ClipboardCapture::NoLog => {
                        if let Some(m) = text_recognizer.find_text_containing(&frame, &export_copied_needles()).into_iter().next() {
                            log_match("Export Copied!", &m, &window, &frame);
                            finish_capture(&app, &activator, &input, &window, &clipboard, &store, &mut seen, &mut state, &mut step_at);
                        } else if let Some(m) = text_recognizer.find_text(&frame, &confirm_needles()).into_iter().next() {
                            log_match("Confirm", &m, &window, &frame);
                            let detail = format!("{} at ({},{}) {}x{}", m.text, m.x, m.y, m.width, m.height);
                            advance(&app, &mut state, CaptureEvent::ConfirmFound, "Confirm dialog found", Some(detail));
                            let (cx, cy) = click_point(&window, &frame, &m);
                            if click_at(&activator, &input, cx, cy).is_ok() {
                                advance(&app, &mut state, CaptureEvent::ConfirmClicked, "Confirm clicked", Some(format!("({}, {})", cx, cy)));
                                step_at = Some(Instant::now());
                            }
                        } else if let Some(t) = step_at {
                            // No clipboard log, no toast, no confirm within the
                            // step timeout — give up on this attempt rather than
                            // spin forever. The passive clipboard poll at the
                            // top of the loop still captures the log if it
                            // lands late.
                            if t.elapsed() > Duration::from_secs(STEP_TIMEOUT_SECS) {
                                warn!("export produced no clipboard log, toast, or confirm; resetting to watching");
                                reset_to_watching(&mut state, &mut step_at);
                            }
                        }
                    }
                }
            }
            CaptureState::ConfirmPressed => {
                // A real confirm dialog was dismissed; finish the sequence.
                if let Some(t) = step_at {
                    if t.elapsed() >= Duration::from_millis(DIALOG_SETTLE_MS) {
                        finish_capture(&app, &activator, &input, &window, &clipboard, &store, &mut seen, &mut state, &mut step_at);
                    }
                }
            }
            CaptureState::AwaitingNextMatch => {
                // Idle after a capture. Re-arm only once the results screen is
                // actually gone, so we never re-click the Battle Log tab (or
                // steal focus) while the player is still looking at it. Require
                // a few consecutive clear reads so a mid-animation frame can't
                // re-arm us early.
                if text_recognizer.find_text(&frame, &battle_log_needles()).is_empty() {
                    results_clear_polls += 1;
                    if results_clear_polls >= REARM_CLEAR_POLLS {
                        results_clear_polls = 0;
                        advance(&app, &mut state, CaptureEvent::ResultsScreenDismissed, "Results screen dismissed; armed for the next match", None);
                    }
                } else {
                    results_clear_polls = 0;
                }
            }
            _ => {}
        }

        // A step that never found its next element within the timeout falls
        // back to scanning for the Battle Log tab again. This covers both the
        // transient `*Detected` states (click never landed) and the `*Opened`/
        // `*Pressed` states (next needle never appeared). `AwaitingNextMatch`
        // is deliberately excluded: it is a resting state, not a step in
        // progress, and must not be reset by a timeout.
        if !matches!(state, CaptureState::AwaitingNextMatch) {
            if let Some(t) = step_at {
                if t.elapsed() > Duration::from_secs(STEP_TIMEOUT_SECS) {
                    warn!(state = %state, "capture step timed out; returning to Watching");
                    reset_to_watching(&mut state, &mut step_at);
                }
            }
        }

        // Poll fast whenever something transient is in flight, and while
        // watching (the region-only OCR is cheap enough to poll aggressively so
        // we catch the results screen before the player clicks Continue).
        // Otherwise idle slowly.
        let transient = matches!(
            state,
            CaptureState::Watching
                | CaptureState::LogTabDetected
                | CaptureState::PanelOpened
                | CaptureState::ExportDetected
                | CaptureState::ExportPressed
                | CaptureState::ConfirmDetected
                | CaptureState::ConfirmPressed
                | CaptureState::CloseDetected
        );
        let poll = if transient { POLL_FAST_MS } else { POLL_MS };
        std::thread::sleep(Duration::from_millis(poll));
    }
}

fn log_match(label: &str, m: &TextMatch, window: &WindowInfo, frame: &Frame) {
    let (cx, cy) = click_point(window, frame, m);
    info!(label, text = %m.text, x = m.x, y = m.y, w = m.width, h = m.height, click_x = cx, click_y = cy, frame_w = frame.width, win_w = window.width, scale = frame_to_point_scale(frame.width, window.width), "button found");
}

/// Resolve a window-relative fraction (for an icon-only control) to absolute
/// screen coordinates. Uses the window's point geometry, so it is independent
/// of the captured frame's pixel density.
fn relative_click(window: &WindowInfo, p: &RelativePoint) -> (i32, i32) {
    let local = Point {
        x: (window.width as f32 * p.x_frac).round() as i32,
        y: (window.height as f32 * p.y_frac).round() as i32,
    };
    let screen = window_to_screen(Point { x: window.x, y: window.y }, local);
    (screen.x, screen.y)
}

/// Focus the game window, then synthesize a click at absolute screen coords.
fn click_at(activator: &MacWindowActivator, input: &EnigoInput, x: i32, y: i32) -> Result<(), String> {
    if let Err(e) = activator.activate_game(GAME_PROCESS_NAME) {
        warn!("failed to focus game window: {e}");
    }
    std::thread::sleep(Duration::from_millis(FOCUS_SETTLE_MS));
    input.click(x, y)
}

fn reset_to_watching(state: &mut CaptureState, step_at: &mut Option<Instant>) {
    *state = CaptureState::Watching;
    *step_at = None;
}

/// Outcome of inspecting the clipboard for a battle log.
#[derive(Debug, PartialEq, Eq)]
enum ClipboardCapture {
    /// A new battle log was written to disk; holds its path.
    Saved(String),
    /// The clipboard holds a battle log we have *already* saved (same match).
    /// This is a success, not a failure: we already have the log.
    AlreadyCaptured,
    /// The clipboard does not currently hold a recognizable battle log.
    NoLog,
}

/// Complete the sequence: dismiss the Battle Log drawer via its close icon,
/// ensure the log that Export put on the clipboard is saved, then cool down.
#[allow(clippy::too_many_arguments)]
fn finish_capture(
    app: &AppHandle,
    activator: &MacWindowActivator,
    input: &EnigoInput,
    window: &WindowInfo,
    clipboard: &ArboardClipboard,
    store: &FsLogStore,
    seen: &mut SeenHashes,
    state: &mut CaptureState,
    step_at: &mut Option<Instant>,
) {
    let (cx, cy) = relative_click(window, &close_icon());
    info!(label = "Close icon", click_x = cx, click_y = cy, "dismissing Battle Log drawer");
    if click_at(activator, input, cx, cy).is_ok() {
        advance(app, state, CaptureEvent::CloseFound, "Close icon targeted", Some(format!("({}, {})", cx, cy)));
        advance(app, state, CaptureEvent::CloseClicked, "Close clicked", Some(format!("({}, {})", cx, cy)));
    }
    // The log may already have been saved by the passive poll or an earlier
    // step. Both "just saved it" and "we already have it" mean this match is
    // captured — only "no log at all" is a failure. Treating an
    // already-captured log as failure is what previously made the engine
    // reset to Watching and re-click the same results screen forever.
    let outcome = capture_clipboard(clipboard, store, seen);
    match &outcome {
        ClipboardCapture::Saved(path) => {
            advance(app, state, CaptureEvent::LogCaptured, "Battle log captured and saved", Some(path.clone()));
        }
        ClipboardCapture::AlreadyCaptured => {
            info!("battle log already captured; resting until the results screen is dismissed");
            advance(app, state, CaptureEvent::LogCaptured, "Battle log already captured", None);
        }
        ClipboardCapture::NoLog => {
            warn!("no battle log on clipboard after export; resetting to watching");
        }
    }
    if is_captured(&outcome) {
        std::thread::sleep(Duration::from_millis(REARM_DELAY_MS));
        // Stay idle on the results screen until the player dismisses it, so we
        // never re-capture the same log or keep yanking the window forward.
        advance(app, state, CaptureEvent::CooldownElapsed, "Waiting for the results screen to be dismissed", None);
        *state = CaptureState::AwaitingNextMatch;
    } else {
        reset_to_watching(state, step_at);
    }
    *step_at = None;
}

/// Whether a "BATTLE LOG" OCR match corresponds to the results-screen tab
/// (which lives in the bottom band of the window) rather than the Battle Log
/// drawer's own tab (top-right, when the drawer is already open).
///
/// The periodic full-frame sweep scans everything, so without this guard a
/// full sweep can match the drawer's header tab at the top of the screen and
/// spuriously re-run the capture flow mid-match. The results-screen tab is a
/// bottom tab-bar control and always sits low in the window (measured y≈81%),
/// while the drawer's tab sits at the very top (y≈6%), so a lower-band
/// threshold cleanly separates the two. Region-based detections are unaffected
/// (the region is already in the bottom band).
fn is_results_screen_tab(m: &TextMatch, frame: &Frame) -> bool {
    if frame.height == 0 {
        return true;
    }
    let center_y = m.y as f32 + m.height as f32 / 2.0;
    center_y >= frame.height as f32 * 0.40
}

/// Whether a clipboard outcome means this match's log is in hand — either just
/// saved, or already saved by an earlier read (the passive clipboard poll often
/// wins the race with the click flow). An already-captured log is a *success*:
/// resetting to Watching on it is what made the engine re-click the same
/// results screen forever.
fn is_captured(outcome: &ClipboardCapture) -> bool {
    matches!(outcome, ClipboardCapture::Saved(_) | ClipboardCapture::AlreadyCaptured)
}

/// Reads the clipboard, validates it looks like a battle log, dedupes, and
/// saves it.
fn capture_clipboard(clipboard: &ArboardClipboard, store: &FsLogStore, seen: &mut SeenHashes) -> ClipboardCapture {
    match clipboard.read_text() {
        Some(text) if looks_like_battle_log(&text) => {
            if seen.insert(&text) {
                match store.save(&text) {
                    Ok(path) => {
                        info!(path = %path, "battle log saved");
                        ClipboardCapture::Saved(path)
                    }
                    Err(e) => {
                        error!("failed to save battle log: {e}");
                        ClipboardCapture::NoLog
                    }
                }
            } else {
                debug!("duplicate battle log ignored");
                ClipboardCapture::AlreadyCaptured
            }
        }
        Some(_) => {
            debug!("clipboard text is not a battle log");
            ClipboardCapture::NoLog
        }
        None => {
            debug!("clipboard is empty");
            ClipboardCapture::NoLog
        }
    }
}

/// Advances the state machine, and on an actual transition logs it and emits a
/// `capture-status` event to the frontend.
fn advance(app: &AppHandle, state: &mut CaptureState, event: CaptureEvent, message: &str, detail: Option<String>) {
    let next_state = next(*state, event);
    if next_state == *state {
        return;
    }
    *state = next_state;
    let status = CaptureStatus::new(next_state, message, detail);
    match event {
        CaptureEvent::WindowLost | CaptureEvent::PermissionDenied => warn!(state = %status.state, "{}", status.message),
        _ => info!(state = %status.state, detail = status.detail.as_deref().unwrap_or(""), "{}", status.message),
    }
    if let Err(e) = app.emit(CAPTURE_STATUS_EVENT, &status) {
        error!("failed to emit capture-status event: {e}");
    }
}

/// Emit a `capture-status` event without a state-machine transition.
fn emit_status(app: &AppHandle, state: &str, message: &str, detail: Option<String>) {
    let status = CaptureStatus::custom(state, message, detail);
    if let Err(e) = app.emit(CAPTURE_STATUS_EVENT, status) {
        warn!("failed to emit capture status: {e}");
    }
}

fn click_point(window: &WindowInfo, frame: &Frame, m: &TextMatch) -> (i32, i32) {
    // OCR coordinates are in captured-frame pixels; window geometry and click
    // targets are in points. On a Retina capture the frame is 2x, so divide
    // before offsetting by the window origin.
    let scale = frame_to_point_scale(frame.width, window.width);
    let center_px = rect_center(Point { x: m.x as i32, y: m.y as i32 }, (m.width, m.height));
    let center_pt = Point {
        x: (center_px.x as f32 / scale).round() as i32,
        y: (center_px.y as f32 / scale).round() as i32,
    };
    let screen = window_to_screen(Point { x: window.x, y: window.y }, center_pt);
    (screen.x, screen.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn already_captured_counts_as_success() {
        // Regression: an already-saved log must be treated as a successful
        // capture (rest on the results screen), NOT as a failure that resets to
        // Watching — that mistake made the engine re-click the same results
        // screen in an infinite loop.
        assert!(is_captured(&ClipboardCapture::Saved("/tmp/a.txt".into())));
        assert!(is_captured(&ClipboardCapture::AlreadyCaptured));
    }

    #[test]
    fn no_log_is_the_only_failure() {
        assert!(!is_captured(&ClipboardCapture::NoLog));
    }

    #[test]
    fn accepts_results_screen_tab_rejects_drawer_tab() {
        // Real frame from a live capture: 2560x1496.
        let frame = Frame { width: 2560, height: 1496, rgba: Vec::new() };
        // Results-screen tab, measured at y=1215 h=19 (bottom band) -> accept.
        let results = TextMatch { text: "BATTLE LOG".into(), x: 1223, y: 1215, width: 114, height: 18 };
        assert!(is_results_screen_tab(&results, &frame));
        // The drawer's own tab, measured at y=92 h=38 (top-right) -> reject.
        let drawer = TextMatch { text: "BATTLE LOG".into(), x: 2072, y: 92, width: 201, height: 38 };
        assert!(!is_results_screen_tab(&drawer, &frame));
    }

    #[test]
    fn degenerate_frame_accepts_anywhere() {
        let frame = Frame { width: 2560, height: 0, rgba: Vec::new() };
        let m = TextMatch { text: "BATTLE LOG".into(), x: 0, y: 0, width: 10, height: 10 };
        assert!(is_results_screen_tab(&m, &frame));
    }
}