//! Read-only results-screen detection.

use crate::buttons::{battle_log_needles, battle_log_region, continue_needles, continue_region, drawer_title_region};
use crate::coords::crop_frame;
use crate::ports::{Frame, TextMatch, TextRecognizer};

/// Consecutive polls with neither signal before results count as gone.
pub const RESULTS_CLEAR_POLLS: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultsChange {
    Appeared,
    Gone,
    Unchanged,
}

/// Debounces the results screen's presence across polls. Pure.
#[derive(Debug, Default)]
pub struct ResultsTracker {
    present: bool,
    clear_polls: u32,
}

impl ResultsTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_present(&self) -> bool {
        self.present
    }

    /// Only the results-screen tab can make results appear; the drawer can also
    /// open mid-match, so it only keeps already-present results alive (opening it
    /// to Export can hide the tab).
    pub fn observe(&mut self, tab_visible: bool, drawer_open: bool) -> ResultsChange {
        if tab_visible || (self.present && drawer_open) {
            self.clear_polls = 0;
            if self.present {
                return ResultsChange::Unchanged;
            }
            self.present = true;
            return ResultsChange::Appeared;
        }
        if !self.present {
            return ResultsChange::Unchanged;
        }
        self.clear_polls += 1;
        if self.clear_polls >= RESULTS_CLEAR_POLLS {
            *self = Self::default();
            return ResultsChange::Gone;
        }
        ResultsChange::Unchanged
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// What the engine needs to know about one captured frame.
pub trait ResultsProbe {
    /// The results-screen Battle Log tab is visible.
    fn tab_visible(&self, frame: &Frame, full_sweep: bool) -> bool;
    /// The Battle Log drawer is open (its title shows at the top).
    fn drawer_open(&self, frame: &Frame) -> bool;
    /// The results screen's Continue label, in frame pixels.
    fn find_continue(&self, frame: &Frame) -> Option<TextMatch>;
}

/// `ResultsProbe` backed by OCR on cropped regions of the frame.
pub struct OcrResultsProbe<T: TextRecognizer> {
    ocr: T,
}

impl<T: TextRecognizer> OcrResultsProbe<T> {
    pub fn new(ocr: T) -> Self {
        Self { ocr }
    }

    fn region_has_battle_log(&self, frame: &Frame, region: crate::coords::RelativeRect) -> bool {
        crop_frame(frame, region)
            .map(|crop| !self.ocr.find_text(&crop.frame, &battle_log_needles()).is_empty())
            .unwrap_or(false)
    }
}

impl<T: TextRecognizer> ResultsProbe for OcrResultsProbe<T> {
    fn tab_visible(&self, frame: &Frame, full_sweep: bool) -> bool {
        if full_sweep {
            // Safety net in case the tab ever renders outside its measured region.
            return self
                .ocr
                .find_text(frame, &battle_log_needles())
                .iter()
                .any(|m| is_results_screen_tab(m, frame));
        }
        self.region_has_battle_log(frame, battle_log_region())
    }

    fn drawer_open(&self, frame: &Frame) -> bool {
        self.region_has_battle_log(frame, drawer_title_region())
    }

    fn find_continue(&self, frame: &Frame) -> Option<TextMatch> {
        let crop = crop_frame(frame, continue_region())?;
        let m = self.ocr.find_text(&crop.frame, &continue_needles()).into_iter().next()?;
        Some(TextMatch { x: m.x + crop.origin_x, y: m.y + crop.origin_y, ..m })
    }
}

/// Whether a "BATTLE LOG" match is the results-screen tab (bottom band, y≈81%)
/// rather than the open drawer's own header tab (top, y≈6%).
pub fn is_results_screen_tab(m: &TextMatch, frame: &Frame) -> bool {
    if frame.height == 0 {
        return true;
    }
    let center_y = m.y as f32 + m.height as f32 / 2.0;
    center_y >= frame.height as f32 * 0.40
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedOcr(Vec<TextMatch>);

    impl TextRecognizer for FixedOcr {
        fn find_text(&self, _frame: &Frame, _needles: &[String]) -> Vec<TextMatch> {
            self.0.clone()
        }

        fn find_text_containing(&self, _frame: &Frame, _needles: &[String]) -> Vec<TextMatch> {
            Vec::new()
        }
    }

    fn frame() -> Frame {
        Frame { width: 256, height: 150, rgba: vec![0u8; 256 * 150 * 4] }
    }

    fn label(text: &str, y: u32) -> TextMatch {
        TextMatch { text: text.into(), x: 100, y, width: 20, height: 4 }
    }

    #[test]
    fn region_hit_means_tab_visible() {
        assert!(OcrResultsProbe::new(FixedOcr(vec![label("Battle Log", 3)])).tab_visible(&frame(), false));
        assert!(!OcrResultsProbe::new(FixedOcr(Vec::new())).tab_visible(&frame(), false));
    }

    #[test]
    fn full_sweep_rejects_the_drawer_header_tab() {
        assert!(!OcrResultsProbe::new(FixedOcr(vec![label("Battle Log", 5)])).tab_visible(&frame(), true));
        assert!(OcrResultsProbe::new(FixedOcr(vec![label("Battle Log", 120)])).tab_visible(&frame(), true));
    }

    #[test]
    fn drawer_title_means_drawer_open() {
        assert!(OcrResultsProbe::new(FixedOcr(vec![label("Battle Log", 3)])).drawer_open(&frame()));
        assert!(!OcrResultsProbe::new(FixedOcr(Vec::new())).drawer_open(&frame()));
    }

    #[test]
    fn find_continue_maps_the_cropped_match_back_to_the_frame() {
        // 256x150 frame: continue_region starts at (round(0.40*256), round(0.90*150)) = (102, 135).
        let probe = OcrResultsProbe::new(FixedOcr(vec![label("Continue", 3), label("Continue", 9)]));
        let m = probe.find_continue(&frame()).unwrap();
        assert_eq!((m.x, m.y), (100 + 102, 3 + 135));
    }

    #[test]
    fn find_continue_is_none_without_a_match() {
        assert!(OcrResultsProbe::new(FixedOcr(Vec::new())).find_continue(&frame()).is_none());
    }

    #[test]
    fn accepts_results_screen_tab_rejects_drawer_tab() {
        let frame = Frame { width: 2560, height: 1496, rgba: Vec::new() };
        let results = TextMatch { text: "BATTLE LOG".into(), x: 1223, y: 1215, width: 114, height: 18 };
        assert!(is_results_screen_tab(&results, &frame));
        let drawer = TextMatch { text: "BATTLE LOG".into(), x: 2072, y: 92, width: 201, height: 38 };
        assert!(!is_results_screen_tab(&drawer, &frame));
    }

    #[test]
    fn degenerate_frame_accepts_anywhere() {
        let frame = Frame { width: 2560, height: 0, rgba: Vec::new() };
        let m = TextMatch { text: "BATTLE LOG".into(), x: 0, y: 0, width: 10, height: 10 };
        assert!(is_results_screen_tab(&m, &frame));
    }

    #[test]
    fn tab_makes_results_appear_once() {
        let mut t = ResultsTracker::new();
        assert_eq!(t.observe(true, false), ResultsChange::Appeared);
        assert_eq!(t.observe(true, false), ResultsChange::Unchanged);
        assert!(t.is_present());
    }

    #[test]
    fn drawer_alone_never_raises_results() {
        let mut t = ResultsTracker::new();
        assert_eq!(t.observe(false, true), ResultsChange::Unchanged);
        assert!(!t.is_present());
    }

    #[test]
    fn drawer_keeps_present_results_alive() {
        let mut t = ResultsTracker::new();
        t.observe(true, false);
        for _ in 0..10 {
            assert_eq!(t.observe(false, true), ResultsChange::Unchanged);
        }
        assert!(t.is_present());
    }

    #[test]
    fn results_go_after_three_clear_polls() {
        let mut t = ResultsTracker::new();
        t.observe(true, false);
        assert_eq!(t.observe(false, false), ResultsChange::Unchanged);
        assert_eq!(t.observe(false, false), ResultsChange::Unchanged);
        assert_eq!(t.observe(false, false), ResultsChange::Gone);
        assert!(!t.is_present());
    }

    #[test]
    fn a_hit_resets_the_clear_count() {
        let mut t = ResultsTracker::new();
        t.observe(true, false);
        t.observe(false, false);
        t.observe(false, false);
        t.observe(false, true);
        assert_eq!(t.observe(false, false), ResultsChange::Unchanged);
        assert_eq!(t.observe(false, false), ResultsChange::Unchanged);
        assert_eq!(t.observe(false, false), ResultsChange::Gone);
    }

    #[test]
    fn reset_forgets_presence() {
        let mut t = ResultsTracker::new();
        t.observe(true, false);
        t.reset();
        assert!(!t.is_present());
        assert_eq!(t.observe(true, false), ResultsChange::Appeared);
    }
}
