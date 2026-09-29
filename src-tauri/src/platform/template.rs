//! Template matching for the capture engine, backed by `imageproc` normalized
//! cross-correlation. Pure image math — no OS calls — so it is unit-testable
//! with synthetic frames.

use image::imageops::{self, FilterType};
use image::{GrayImage, Luma, RgbaImage};
use imageproc::template_matching::{find_extremes, match_template, MatchTemplateMethod};

use crate::coords::{crop_frame, Point, RelativeRect};
use crate::ports::{Frame, TemplateMatcher};

/// Minimum normalized cross-correlation score (0..=1) to accept a match.
const SCORE_THRESHOLD: f32 = 0.8;

/// Template matcher backed by `imageproc`.
pub struct ImageprocTemplateMatcher;

impl TemplateMatcher for ImageprocTemplateMatcher {
    fn find(
        &self,
        frame: &Frame,
        template: &RgbaImage,
        template_scale: f32,
        frame_scale: f32,
        search: Option<RelativeRect>,
    ) -> Option<Point> {
        if template.width() == 0 || template.height() == 0 {
            return None;
        }

        // Confine the search to the requested region (if any), matching in the
        // crop and offsetting the result back to full-frame coordinates.
        let cropped: Frame;
        let (work, origin_x, origin_y): (&Frame, u32, u32) =
            match search.and_then(|r| crop_frame(frame, r)) {
                Some(crop) => {
                    cropped = crop.frame;
                    (&cropped, crop.origin_x, crop.origin_y)
                }
                None => (frame, 0, 0),
            };

        let frame_gray = frame_to_luma(work);
        let template_gray = resize_to(&rgba_to_luma(template), template_scale, frame_scale);
        if template_gray.width() == 0
            || template_gray.height() == 0
            || template_gray.width() >= frame_gray.width()
            || template_gray.height() >= frame_gray.height()
        {
            return None;
        }

        // Naive NCC is O(W*H*w*h); at full resolution these sizes stall the
        // capture loop for seconds. Downscale both by 2 (16x fewer ops) and map
        // the coarse peak back to full-frame pixels. A +/-2px error is fine for
        // a click target.
        let ds = 2u32;
        let (small_frame, small_template) = downsample(&frame_gray, &template_gray, ds);
        if small_template.width() < 2
            || small_template.height() < 2
            || small_template.width() >= small_frame.width()
            || small_template.height() >= small_frame.height()
        {
            return None;
        }

        let heatmap = match_template(
            &small_frame,
            &small_template,
            MatchTemplateMethod::CrossCorrelationNormalized,
        );
        let extremes = find_extremes(&heatmap);
        let center = center_from(
            extremes.max_value,
            extremes.max_value_location,
            small_template.width(),
            small_template.height(),
            SCORE_THRESHOLD,
        )?;
        Some(Point {
            x: center.x * ds as i32 + origin_x as i32,
            y: center.y * ds as i32 + origin_y as i32,
        })
    }
}

/// Uniform downscale of both the search image and the template by `ds`, so the
/// correlation runs over ~`ds^2` fewer pixels.
fn downsample(img: &GrayImage, template: &GrayImage, ds: u32) -> (GrayImage, GrayImage) {
    (
        imageops::resize(img, img.width() / ds, img.height() / ds, FilterType::Triangle),
        imageops::resize(template, template.width() / ds, template.height() / ds, FilterType::Triangle),
    )
}

/// Decide a click point from a match score. The score must reach `threshold` to
/// count; a `NaN` score (zero-variance region) is a non-match.
fn center_from(
    max_value: f32,
    max_location: (u32, u32),
    template_w: u32,
    template_h: u32,
    threshold: f32,
) -> Option<Point> {
    if max_value.is_nan() || max_value < threshold {
        return None;
    }
    Some(Point {
        x: (max_location.0 + template_w / 2) as i32,
        y: (max_location.1 + template_h / 2) as i32,
    })
}

/// Resize a gray template so its pixel density matches the frame's density.
/// No-op when the densities agree or either is non-positive (defensive).
fn resize_to(template: &GrayImage, template_scale: f32, frame_scale: f32) -> GrayImage {
    if template_scale <= 0.0 || frame_scale <= 0.0 {
        return template.clone();
    }
    let factor = frame_scale / template_scale;
    if (factor - 1.0).abs() < 0.05 {
        return template.clone();
    }
    let new_w = (template.width() as f32 * factor).round().max(1.0) as u32;
    let new_h = (template.height() as f32 * factor).round().max(1.0) as u32;
    imageops::resize(template, new_w, new_h, FilterType::Triangle)
}

fn frame_to_luma(frame: &Frame) -> GrayImage {
    GrayImage::from_fn(frame.width, frame.height, |x, y| {
        let i = ((y * frame.width + x) * 4) as usize;
        Luma([luma_u8(frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2])])
    })
}

fn rgba_to_luma(template: &RgbaImage) -> GrayImage {
    GrayImage::from_fn(template.width(), template.height(), |x, y| {
        let p = template.get_pixel(x, y).0;
        Luma([luma_u8(p[0], p[1], p[2])])
    })
}

fn luma_u8(r: u8, g: u8, b: u8) -> u8 {
    (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_from_accepts_score_at_threshold() {
        let p = center_from(0.8, (16, 16), 8, 8, 0.8).unwrap();
        assert_eq!(p, Point { x: 20, y: 20 });
    }

    #[test]
    fn center_from_rejects_below_threshold() {
        assert!(center_from(0.79, (16, 16), 8, 8, 0.8).is_none());
    }

    #[test]
    fn center_from_rejects_nan() {
        assert!(center_from(f32::NAN, (16, 16), 8, 8, 0.8).is_none());
    }

    #[test]
    fn resize_to_halves_when_template_is_2x_and_frame_1x() {
        let template = GrayImage::from_fn(16, 16, |_, _| Luma([0u8]));
        let resized = resize_to(&template, 2.0, 1.0);
        assert_eq!((resized.width(), resized.height()), (8, 8));
    }

    #[test]
    fn resize_to_is_noop_when_scales_match() {
        let template = GrayImage::from_fn(16, 16, |_, _| Luma([0u8]));
        let resized = resize_to(&template, 2.0, 2.0);
        assert_eq!((resized.width(), resized.height()), (16, 16));
    }

    #[test]
    fn resize_to_is_noop_for_non_positive_scales() {
        let template = GrayImage::from_fn(16, 16, |_, _| Luma([0u8]));
        assert_eq!((resize_to(&template, 0.0, 1.0).width()), 16);
        assert_eq!((resize_to(&template, 1.0, 0.0).height()), 16);
    }

    /// A 40x40 frame whose background is a gradient (non-zero variance
    /// everywhere) with an 8x8 checkerboard embedded at `(tx, ty)`. Returns the
    /// frame, the 8x8 checkerboard template, and the embedded top-left.
    fn embedded_pattern_frame() -> (Frame, RgbaImage, (u32, u32)) {
        let (w, h) = (40u32, 40u32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let v = ((x + y) % 251) as u8;
                let i = ((y * w + x) * 4) as usize;
                rgba[i] = v;
                rgba[i + 1] = v;
                rgba[i + 2] = v;
                rgba[i + 3] = 255;
            }
        }
        let (tx, ty) = (16u32, 16u32);
        let ts = 8u32;
        let mut template_pixels = vec![0u8; (ts * ts * 4) as usize];
        for yy in 0..ts {
            for xx in 0..ts {
                // Low-frequency quadrant pattern (white diagonal, black
                // anti-diagonal) so the template survives resizing without
                // aliasing to a uniform grey like a 1px checkerboard would.
                let v: u8 = if (xx < 4) == (yy < 4) { 255 } else { 0 };
                let ti = ((yy * ts + xx) * 4) as usize;
                template_pixels[ti] = v;
                template_pixels[ti + 1] = v;
                template_pixels[ti + 2] = v;
                template_pixels[ti + 3] = 255;
                let fi = (((ty + yy) * w + (tx + xx)) * 4) as usize;
                rgba[fi] = v;
                rgba[fi + 1] = v;
                rgba[fi + 2] = v;
                rgba[fi + 3] = 255;
            }
        }
        let frame = Frame { width: w, height: h, rgba };
        let template = RgbaImage::from_raw(ts, ts, template_pixels).unwrap();
        (frame, template, (tx, ty))
    }

    /// Nearest-neighbour upscale of a frame by an integer factor, preserving the
    /// embedded pattern's location (scaled by the factor).
    fn upscale_frame(frame: &Frame, k: u32) -> Frame {
        let (w, h) = (frame.width * k, frame.height * k);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let si = (((y / k) * frame.width + (x / k)) * 4) as usize;
                let di = ((y * w + x) * 4) as usize;
                rgba[di..di + 4].copy_from_slice(&frame.rgba[si..si + 4]);
            }
        }
        Frame { width: w, height: h, rgba }
    }

    #[test]
    fn finds_embedded_template_center_at_same_scale() {
        let (frame, template, (tx, ty)) = embedded_pattern_frame();
        let m = ImageprocTemplateMatcher;
        let p = m.find(&frame, &template, 1.0, 1.0, None).unwrap();
        assert_eq!(p.x, (tx + 4) as i32);
        assert_eq!(p.y, (ty + 4) as i32);
    }

    #[test]
    fn finds_embedded_template_when_upscaled_to_frame_density() {
        // A 1x template matched against a 2x frame exercises the resize path
        // (upscaling preserves the pattern, unlike downscaling a 1px checker).
        let (frame1x, template1x, (tx, ty)) = embedded_pattern_frame();
        let frame2x = upscale_frame(&frame1x, 2);
        let m = ImageprocTemplateMatcher;
        let p = m.find(&frame2x, &template1x, 1.0, 2.0, None).unwrap();
        assert_eq!(p.x, (tx * 2 + 8) as i32);
        assert_eq!(p.y, (ty * 2 + 8) as i32);
    }

    #[test]
    fn rejects_empty_template() {
        let (frame, _template, _) = embedded_pattern_frame();
        let m = ImageprocTemplateMatcher;
        let empty = RgbaImage::new(0, 0);
        assert!(m.find(&frame, &empty, 1.0, 1.0, None).is_none());
    }

    #[test]
    fn rejects_template_larger_than_frame() {
        let (frame, _template, _) = embedded_pattern_frame();
        let m = ImageprocTemplateMatcher;
        let big = RgbaImage::new(40, 40);
        assert!(m.find(&frame, &big, 1.0, 1.0, None).is_none());
    }

    #[test]
    fn confines_search_to_region_and_offsets_result() {
        let (frame, template, (tx, ty)) = embedded_pattern_frame();
        let m = ImageprocTemplateMatcher;

        // The embedded 8x8 block lives at x=16..24, y=16..24 (fraction
        // 0.4..0.6). A search confined to the bottom-right corner must not
        // find it.
        let corner = RelativeRect { x0: 0.6, y0: 0.6, x1: 1.0, y1: 1.0 };
        assert!(m.find(&frame, &template, 1.0, 1.0, Some(corner)).is_none());

        // A region that contains the block finds it, reporting full-frame coords.
        let containing = RelativeRect { x0: 0.0, y0: 0.0, x1: 0.75, y1: 1.0 };
        let p = m.find(&frame, &template, 1.0, 1.0, Some(containing)).unwrap();
        assert_eq!(p.x, (tx + 4) as i32);
        assert_eq!(p.y, (ty + 4) as i32);
    }
}
