/// A point in screen or window-local pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

use crate::ports::Frame;

/// Convert a window-local point to screen coordinates by adding the window origin.
pub fn window_to_screen(window_origin: Point, local: Point) -> Point {
    Point { x: window_origin.x + local.x, y: window_origin.y + local.y }
}

/// Center of a rectangle given its top-left corner and pixel size.
pub fn rect_center(top_left: Point, size: (u32, u32)) -> Point {
    Point { x: top_left.x + (size.0 as i32) / 2, y: top_left.y + (size.1 as i32) / 2 }
}

/// Uniform scale from captured-frame pixels to window points. Screen capture on
/// a Retina display returns 2x pixels while the window geometry (and hence
/// `window.x`/`window.y` and click targets) is in points, so frame-local pixel
/// coordinates must be divided by this before being offset by the window origin.
/// Returns 1.0 when the frame is narrower than the window (defensive).
pub fn frame_to_point_scale(frame_px: u32, window_pt: u32) -> f32 {
    if frame_px == 0 || window_pt == 0 || frame_px <= window_pt {
        1.0
    } else {
        frame_px as f32 / window_pt as f32
    }
}

/// A rectangle expressed as fractions of the frame (0.0..1.0) on each axis.
/// Used to crop a small region of interest before OCR, which is far cheaper
/// than recognizing the whole window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelativeRect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

/// A cropped sub-frame plus the pixel offset of its top-left corner within the
/// original frame, so region-local OCR matches can be mapped back to
/// full-frame coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crop {
    pub frame: Frame,
    pub origin_x: u32,
    pub origin_y: u32,
}

/// Crop `frame` to `rect` (fractions of the frame size). Returns `None` if the
/// rect is degenerate or lies entirely outside the frame.
pub fn crop_frame(frame: &Frame, rect: RelativeRect) -> Option<Crop> {
    let fx = |v: f32, max: u32| ((v * max as f32).round() as i64).clamp(0, max as i64) as u32;
    let x0 = fx(rect.x0, frame.width);
    let x1 = fx(rect.x1, frame.width);
    let y0 = fx(rect.y0, frame.height);
    let y1 = fx(rect.y1, frame.height);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let w = x1 - x0;
    let h = y1 - y0;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in y0..y1 {
        let start = ((y * frame.width + x0) * 4) as usize;
        let len = (w * 4) as usize;
        rgba.extend_from_slice(&frame.rgba[start..start + len]);
    }
    Some(Crop { frame: Frame { width: w, height: h, rgba }, origin_x: x0, origin_y: y0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_to_screen_adds_origin() {
        let origin = Point { x: 100, y: 50 };
        let local = Point { x: 20, y: 30 };
        assert_eq!(window_to_screen(origin, local), Point { x: 120, y: 80 });
    }

    #[test]
    fn rect_center_of_even_size() {
        let top_left = Point { x: 0, y: 0 };
        assert_eq!(rect_center(top_left, (100, 50)), Point { x: 50, y: 25 });
    }

    #[test]
    fn rect_center_with_offset_top_left() {
        let top_left = Point { x: 10, y: 20 };
        assert_eq!(rect_center(top_left, (10, 10)), Point { x: 15, y: 25 });
    }

    #[test]
    fn scale_is_one_on_non_retina() {
        let s = frame_to_point_scale(1280, 1280);
        assert!((s - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn scale_is_two_on_retina_capture() {
        let s = frame_to_point_scale(2560, 1280);
        assert!((s - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn scale_defensive_defaults_to_one() {
        assert!((frame_to_point_scale(0, 1280) - 1.0).abs() < f32::EPSILON);
        assert!((frame_to_point_scale(2560, 0) - 1.0).abs() < f32::EPSILON);
        assert!((frame_to_point_scale(100, 1280) - 1.0).abs() < f32::EPSILON);
    }

    fn solid_frame(w: u32, h: u32) -> Frame {
        Frame { width: w, height: h, rgba: vec![7u8; (w * h * 4) as usize] }
    }

    #[test]
    fn crop_extracts_sub_rectangle() {
        // 2x1 pixels: left pixel red, right pixel blue.
        let f = Frame { width: 2, height: 1, rgba: vec![255, 0, 0, 255, 0, 0, 255, 255] };
        let crop = crop_frame(&f, RelativeRect { x0: 0.5, y0: 0.0, x1: 1.0, y1: 1.0 }).unwrap();
        assert_eq!(crop.frame.width, 1);
        assert_eq!(crop.frame.height, 1);
        assert_eq!(crop.origin_x, 1);
        assert_eq!(crop.origin_y, 0);
        assert_eq!(crop.frame.rgba, vec![0, 0, 255, 255]);
    }

    #[test]
    fn crop_rejects_degenerate_rect() {
        let f = solid_frame(10, 10);
        assert!(crop_frame(&f, RelativeRect { x0: 0.5, y0: 0.0, x1: 0.5, y1: 1.0 }).is_none());
        assert!(crop_frame(&f, RelativeRect { x0: 0.0, y0: 0.5, x1: 1.0, y1: 0.5 }).is_none());
    }

    #[test]
    fn battle_log_region_contains_the_measured_tab() {
        // Measured live: tab top-left at x=1220, y=1213, ~127x19 in a
        // 2560x1496 frame, consistently across matches.
        let f = solid_frame(2560, 1496);
        let crop = crop_frame(&f, crate::buttons::battle_log_region()).unwrap();
        // The tab box must be fully inside the cropped region.
        let (x0, y0) = (crop.origin_x, crop.origin_y);
        let (x1, y1) = (x0 + crop.frame.width, y0 + crop.frame.height);
        assert!((1220..=1347).all(|x| x0 <= x && x <= x1), "x range {x0}..{x1} must contain 1220..=1347");
        assert!((1213..=1232).all(|y| y0 <= y && y <= y1), "y range {y0}..{y1} must contain 1213..=1232");
    }
}
