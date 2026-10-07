//! Pure overlay geometry: where the capture drawer and Continue shield sit for
//! a given game window. All values are in points (window geometry units), never
//! captured-frame pixels.

use crate::coords::frame_to_point_scale;
use crate::ports::{Frame, TextMatch, WindowInfo};

/// A rectangle in points, top-left origin. Screen-global or window-local
/// depending on the function that produced it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub const INSET_PT: f64 = 12.0;
pub const HANDLE_PT: (f64, f64) = (140.0, 32.0);
pub const DRAWER_MAX_WIDTH_PT: f64 = 380.0;
pub const DRAWER_MAX_WIDTH_FRAC: f64 = 0.4;
pub const SHIELD_PAD_PT: (f64, f64) = (16.0, 10.0);
/// Big enough for the shield's message and Skip button, whatever the label width.
pub const SHIELD_MIN_PT: (f64, f64) = (240.0, 44.0);

/// Screen rect for the capture drawer window: the handle at the game's
/// bottom-left when closed, a full-height left-edge drawer when open.
pub fn drawer_rect(game: &WindowInfo, open: bool) -> ScreenRect {
    let (gx, gy) = (f64::from(game.x), f64::from(game.y));
    let (gw, gh) = (f64::from(game.width), f64::from(game.height));
    if open {
        let width = DRAWER_MAX_WIDTH_PT.min(gw * DRAWER_MAX_WIDTH_FRAC);
        return ScreenRect { x: gx, y: gy, width, height: gh };
    }
    let (width, height) = HANDLE_PT;
    ScreenRect { x: gx + INSET_PT, y: gy + gh - INSET_PT - height, width, height }
}

/// Window-local shield rect for an OCR'd Continue label: padded, at least
/// `SHIELD_MIN_PT`, centered on the label.
pub fn shield_rect(game: &WindowInfo, frame: &Frame, m: &TextMatch) -> ScreenRect {
    let scale = f64::from(frame_to_point_scale(frame.width, game.width));
    let (lx, ly) = (f64::from(m.x) / scale, f64::from(m.y) / scale);
    let (lw, lh) = (f64::from(m.width) / scale, f64::from(m.height) / scale);
    let width = (lw + 2.0 * SHIELD_PAD_PT.0).max(SHIELD_MIN_PT.0);
    let height = (lh + 2.0 * SHIELD_PAD_PT.1).max(SHIELD_MIN_PT.1);
    ScreenRect {
        x: lx + lw / 2.0 - width / 2.0,
        y: ly + lh / 2.0 - height / 2.0,
        width,
        height,
    }
}

/// Offset a window-local rect by the game window origin.
pub fn to_screen(game: &WindowInfo, local: ScreenRect) -> ScreenRect {
    ScreenRect { x: local.x + f64::from(game.x), y: local.y + f64::from(game.y), ..local }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> WindowInfo {
        WindowInfo { title: "Pokemon TCG Live".into(), pid: Some(4242), x: 100, y: 50, width: 1280, height: 748 }
    }

    #[test]
    fn closed_drawer_is_a_handle_at_the_bottom_left() {
        assert_eq!(drawer_rect(&game(), false), ScreenRect { x: 112.0, y: 754.0, width: 140.0, height: 32.0 });
    }

    #[test]
    fn open_drawer_is_full_height_on_the_left_edge() {
        assert_eq!(drawer_rect(&game(), true), ScreenRect { x: 100.0, y: 50.0, width: 380.0, height: 748.0 });
    }

    #[test]
    fn open_drawer_width_is_capped_by_the_window() {
        let g = WindowInfo { title: "x".into(), pid: Some(1), x: 0, y: 0, width: 800, height: 500 };
        assert_eq!(drawer_rect(&g, true).width, 320.0);
    }

    #[test]
    fn shield_divides_retina_scale_and_enforces_minimum() {
        let frame = Frame { width: 2560, height: 1496, rgba: Vec::new() };
        let m = TextMatch { text: "Continue".into(), x: 2000, y: 1300, width: 200, height: 40 };
        assert_eq!(shield_rect(&game(), &frame, &m), ScreenRect { x: 930.0, y: 638.0, width: 240.0, height: 44.0 });
    }

    #[test]
    fn shield_is_identical_at_1x() {
        let frame = Frame { width: 1280, height: 748, rgba: Vec::new() };
        let m = TextMatch { text: "Continue".into(), x: 1000, y: 650, width: 100, height: 20 };
        assert_eq!(shield_rect(&game(), &frame, &m), ScreenRect { x: 930.0, y: 638.0, width: 240.0, height: 44.0 });
    }

    #[test]
    fn shield_pads_a_wide_label_beyond_the_minimum() {
        let frame = Frame { width: 1280, height: 748, rgba: Vec::new() };
        let m = TextMatch { text: "Fortfahren".into(), x: 1000, y: 650, width: 300, height: 30 };
        assert_eq!(shield_rect(&game(), &frame, &m), ScreenRect { x: 984.0, y: 640.0, width: 332.0, height: 50.0 });
    }

    #[test]
    fn to_screen_adds_the_window_origin() {
        let local = ScreenRect { x: 10.0, y: 20.0, width: 30.0, height: 40.0 };
        assert_eq!(to_screen(&game(), local), ScreenRect { x: 110.0, y: 70.0, width: 30.0, height: 40.0 });
    }
}
