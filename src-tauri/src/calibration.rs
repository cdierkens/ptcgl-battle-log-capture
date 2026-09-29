//! Calibration reference templates captured from the live client in discover
//! mode. A template is the crop of a screen region plus the pixel density
//! (frame px per window pt) it was captured at, so matching can rescale it to
//! any capture density. Pure filesystem + image logic, unit-testable with a
//! temp directory.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::{ImageFormat, RgbaImage};

use crate::buttons::export_icon_region;
use crate::coords::{crop_frame, RelativeRect};
use crate::ports::Frame;
use crate::settings;

pub const EXPORT_ICON_FILE: &str = "export_icon.png";
const EXPORT_ICON_SCALE_FILE: &str = "export_icon.scale";

/// A captured reference template plus the pixel density it was captured at.
#[derive(Debug, Clone, PartialEq)]
pub struct Calibration {
    pub template: RgbaImage,
    /// Frame pixels per window point at capture time (1.0 = non-Retina, 2.0 = Retina).
    pub scale: f32,
}

/// Directory that holds reference templates, under the app base dir.
pub fn calibration_dir() -> PathBuf {
    settings::base_dir().join("calibration")
}

/// Crop `frame` to `rect` as an RGBA image. `None` when the rect is degenerate
/// or lies outside the frame.
pub fn crop_to_rgba(frame: &Frame, rect: RelativeRect) -> Option<RgbaImage> {
    let crop = crop_frame(frame, rect)?;
    RgbaImage::from_raw(crop.frame.width, crop.frame.height, crop.frame.rgba)
}

/// Discover and save the Export-icon template from `frame` into the default
/// calibration dir, returning the written PNG path.
pub fn save_export_icon(frame: &Frame, scale: f32) -> Result<PathBuf, String> {
    save_export_icon_to(&calibration_dir(), frame, scale)
}

/// Load the calibrated Export-icon template, or `None` if none was saved (or it
/// is unreadable/corrupt).
pub fn load_export_icon() -> Option<Calibration> {
    load_export_icon_from(&calibration_dir())
}

fn save_export_icon_to(dir: &Path, frame: &Frame, scale: f32) -> Result<PathBuf, String> {
    let template = crop_to_rgba(frame, export_icon_region())
        .ok_or_else(|| "export icon region is degenerate or outside the frame".to_string())?;

    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let png_path = dir.join(EXPORT_ICON_FILE);
    let png = encode_png(&template)?;
    fs::write(&png_path, png).map_err(|e| e.to_string())?;
    fs::write(dir.join(EXPORT_ICON_SCALE_FILE), scale.to_string()).map_err(|e| e.to_string())?;
    Ok(png_path)
}

fn load_export_icon_from(dir: &Path) -> Option<Calibration> {
    let png_path = dir.join(EXPORT_ICON_FILE);
    let template = image::open(&png_path).ok()?.to_rgba8();
    let scale = fs::read_to_string(dir.join(EXPORT_ICON_SCALE_FILE))
        .ok()
        .and_then(|s| s.trim().parse::<f32>().ok())
        .unwrap_or(1.0);
    Some(Calibration { template, scale })
}

fn encode_png(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!("ptcgl-calibration-{tag}-{nanos}"));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn solid_frame(w: u32, h: u32, value: u8) -> Frame {
        Frame { width: w, height: h, rgba: vec![value; (w * h * 4) as usize] }
    }

    #[test]
    fn crop_extracts_expected_sub_image() {
        // 4x2 frame: rows [red, green, blue, white] then [black, red, green, blue].
        let frame = Frame {
            width: 4,
            height: 2,
            rgba: vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
                0, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255,
            ],
        };
        let img = crop_to_rgba(&frame, RelativeRect { x0: 0.25, y0: 0.0, x1: 0.75, y1: 1.0 }).unwrap();
        assert_eq!((img.width(), img.height()), (2, 2));
        // Second column: green (top), red (bottom).
        assert_eq!(img.get_pixel(0, 0).0, [0, 255, 0, 255]);
        assert_eq!(img.get_pixel(0, 1).0, [255, 0, 0, 255]);
    }

    #[test]
    fn crop_returns_none_for_degenerate_rect() {
        let frame = solid_frame(10, 10, 7);
        assert!(crop_to_rgba(&frame, RelativeRect { x0: 0.5, y0: 0.0, x1: 0.5, y1: 1.0 }).is_none());
        assert!(crop_to_rgba(&frame, RelativeRect { x0: 0.0, y0: 0.5, x1: 1.0, y1: 0.5 }).is_none());
    }

    #[test]
    fn load_is_none_before_any_save() {
        let dir = TempDir::new("empty");
        assert!(load_export_icon_from(dir.path()).is_none());
    }

    #[test]
    fn save_then_load_round_trips_template_and_scale() {
        let dir = TempDir::new("roundtrip");
        let frame = solid_frame(2000, 1200, 9);
        let saved = save_export_icon_to(dir.path(), &frame, 2.0).unwrap();
        assert!(saved.ends_with(EXPORT_ICON_FILE));

        let cal = load_export_icon_from(dir.path()).unwrap();
        assert_eq!(cal.scale, 2.0);
        assert!(cal.template.width() > 0 && cal.template.height() > 0);
    }

    #[test]
    fn load_defaults_scale_to_one_when_scale_file_missing() {
        let dir = TempDir::new("no-scale");
        let frame = solid_frame(2000, 1200, 9);
        save_export_icon_to(dir.path(), &frame, 1.0).unwrap();
        fs::remove_file(dir.path().join(EXPORT_ICON_SCALE_FILE)).unwrap();
        let cal = load_export_icon_from(dir.path()).unwrap();
        assert_eq!(cal.scale, 1.0);
    }
}
