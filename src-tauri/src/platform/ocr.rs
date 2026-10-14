//! Text recognition for the capture engine.
//!
//! - macOS: Vision framework (VNRecognizeTextRequest)
//! - Windows: Windows.Media.Ocr (built-in, Windows 10+)
//! - Other: no-op stub

#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;
#[cfg(target_os = "macos")]
use objc2::AnyThread;
#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSData, NSDictionary};
#[cfg(target_os = "macos")]
use objc2_vision::{
    VNImageOption, VNImageRequestHandler, VNRecognizeTextRequest, VNRequest, VNRequestTextRecognitionLevel,
};

#[cfg(target_os = "windows")]
use windows::{
    Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine,
    Storage::Streams::{Buffer, DataWriter},
};

#[cfg(target_os = "macos")]
use crate::bmp::encode_bmp;
use crate::ports::{Frame, TextMatch, TextRecognizer};

/// Platform-specific text recognizer.
/// - macOS: Vision framework
/// - Windows: Windows.Media.Ocr
/// - Other: stub
#[derive(Clone, Copy)]
pub struct PlatformTextRecognizer;

#[cfg(target_os = "macos")]
impl TextRecognizer for PlatformTextRecognizer {
    fn find_text(&self, frame: &Frame, needles: &[String]) -> Vec<TextMatch> {
        VisionTextRecognizer.scan(frame, needles, MatchKind::Exact)
    }

    fn find_text_containing(&self, frame: &Frame, needles: &[String]) -> Vec<TextMatch> {
        VisionTextRecognizer.scan(frame, needles, MatchKind::Substring)
    }

    fn warm_up(&self) {
        VisionTextRecognizer.warm_up();
    }
}

#[cfg(target_os = "windows")]
impl TextRecognizer for PlatformTextRecognizer {
    fn find_text(&self, frame: &Frame, needles: &[String]) -> Vec<TextMatch> {
        WindowsOcrRecognizer.find_text_exact(frame, needles)
    }

    fn find_text_containing(&self, frame: &Frame, needles: &[String]) -> Vec<TextMatch> {
        WindowsOcrRecognizer.find_text_containing(frame, needles)
    }

    fn warm_up(&self) {
        WindowsOcrRecognizer.warm_up();
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl TextRecognizer for PlatformTextRecognizer {
    fn find_text(&self, _frame: &Frame, _needles: &[String]) -> Vec<TextMatch> {
        Vec::new()
    }

    fn find_text_containing(&self, _frame: &Frame, _needles: &[String]) -> Vec<TextMatch> {
        Vec::new()
    }
}

// ===== macOS Vision Implementation =====

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
pub struct VisionTextRecognizer;

#[cfg(target_os = "macos")]
impl VisionTextRecognizer {
    fn scan(&self, frame: &Frame, needles: &[String], kind: MatchKind) -> Vec<TextMatch> {
        let Some(bmp) = encode_bmp(frame) else {
            return Vec::new();
        };

        let data = NSData::with_bytes(&bmp);
        let options = NSDictionary::<VNImageOption, AnyObject>::new();
        let handler = VNImageRequestHandler::initWithData_options(VNImageRequestHandler::alloc(), &data, &options);

        let request = VNRecognizeTextRequest::new();
        request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
        request.setUsesLanguageCorrection(false);

        let requests = NSArray::<VNRequest>::from_slice(&[&*request]);
        if handler.performRequests_error(&requests).is_err() {
            return Vec::new();
        }
        let Some(results) = request.results() else {
            return Vec::new();
        };

        let normalized_needles: Vec<String> = needles.iter().map(|n| normalize(n)).collect();
        let fw = frame.width as f32;
        let fh = frame.height as f32;
        let mut matches = Vec::new();

        for observation in results.iter() {
            let Some(top) = observation.topCandidates(1).iter().next() else {
                continue;
            };
            let text = top.string().to_string();
            let hit = match kind {
                MatchKind::Exact => matches_needle(&text, &normalized_needles),
                MatchKind::Substring => matches_substring(&text, &normalized_needles),
            };
            if !hit {
                continue;
            }
            let rect = unsafe { observation.boundingBox() };
            let w = (rect.size.width as f32 * fw).round().max(0.0) as u32;
            let h = (rect.size.height as f32 * fh).round().max(0.0) as u32;
            if w == 0 || h == 0 {
                continue;
            }
            let x = (rect.origin.x as f32 * fw).round().max(0.0) as u32;
            let y = ((1.0 - rect.origin.y as f32 - rect.size.height as f32) * fh).round().max(0.0) as u32;
            matches.push(TextMatch { text, x, y, width: w, height: h });
        }

        matches
    }

    fn warm_up(&self) {
        let blank = Frame { width: 64, height: 64, rgba: vec![255; 64 * 64 * 4] };
        self.scan(&blank, &[], MatchKind::Exact);
    }
}

#[cfg(target_os = "macos")]
impl TextRecognizer for VisionTextRecognizer {
    fn find_text(&self, frame: &Frame, needles: &[String]) -> Vec<TextMatch> {
        self.scan(frame, needles, MatchKind::Exact)
    }

    fn find_text_containing(&self, frame: &Frame, needles: &[String]) -> Vec<TextMatch> {
        self.scan(frame, needles, MatchKind::Substring)
    }

    fn warm_up(&self) {
        self.warm_up();
    }
}

// ===== Windows OCR Implementation =====

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
struct WindowsOcrRecognizer;

#[cfg(target_os = "windows")]
impl WindowsOcrRecognizer {
    fn find_text_exact(&self, frame: &Frame, needles: &[String]) -> Vec<TextMatch> {
        self.scan(frame, needles, MatchKind::Exact)
    }

    fn find_text_containing(&self, frame: &Frame, needles: &[String]) -> Vec<TextMatch> {
        self.scan(frame, needles, MatchKind::Substring)
    }

    fn warm_up(&self) {
        let blank = Frame { width: 64, height: 64, rgba: vec![255; 64 * 64 * 4] };
        let _ = self.scan(&blank, &[], MatchKind::Exact);
    }

    fn scan(&self, frame: &Frame, needles: &[String], kind: MatchKind) -> Vec<TextMatch> {
        let software_bitmap = match frame_to_software_bitmap(frame) {
            Ok(bmp) => bmp,
            Err(_) => return Vec::new(),
        };

        let engine = match OcrEngine::TryCreateFromUserProfileLanguages() {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        let result = match engine.RecognizeAsync(&software_bitmap) {
            Ok(op) => match op.get() {
                Ok(r) => r,
                Err(_) => return Vec::new(),
            },
            Err(_) => return Vec::new(),
        };

        let lines = match result.GetLines() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };

        let normalized_needles: Vec<String> = needles.iter().map(|n| normalize(n)).collect();
        let fw = frame.width as f32;
        let fh = frame.height as f32;
        let mut matches = Vec::new();

        for i in 0..lines.Size() {
            let line = match lines.GetAt(i) {
                Ok(l) => l,
                Err(_) => continue,
            };
            let text = match line.GetText() {
                Ok(t) => t.to_string(),
                Err(_) => continue,
            };

            let hit = match kind {
                MatchKind::Exact => matches_needle(&text, &normalized_needles),
                MatchKind::Substring => matches_substring(&text, &normalized_needles),
            };
            if !hit {
                continue;
            }

            let rect = match line.GetBoundingRect() {
                Ok(r) => r,
                Err(_) => continue,
            };

            let x = rect.X as u32;
            let y = rect.Y as u32;
            let w = rect.Width as u32;
            let h = rect.Height as u32;

            if w == 0 || h == 0 {
                continue;
            }

            matches.push(TextMatch { text, x, y, width: w, height: h });
        }

        matches
    }
}

// Convert RGBA Frame to Windows SoftwareBitmap (BGRA8)
#[cfg(target_os = "windows")]
fn frame_to_software_bitmap(frame: &Frame) -> windows::core::Result<SoftwareBitmap> {
    let width = frame.width;
    let height = frame.height;
    let rgba = &frame.rgba;

    // Windows.Media.Ocr expects BGRA8 format
    let mut bgra = Vec::with_capacity(rgba.len());
    for chunk in rgba.chunks_exact(4) {
        bgra.push(chunk[2]); // B
        bgra.push(chunk[1]); // G
        bgra.push(chunk[0]); // R
        bgra.push(chunk[3]); // A
    }

    // Use SoftwareBitmap::CreateCopyFromBuffer with a buffer created from raw bytes
    // to avoid DataReader/Buffer version conflicts
    use windows::Storage::Streams::{Buffer, DataWriter, IBuffer};
    
    let buffer = Buffer::Create(bgra.len() as u32)?;
    {
        let data_writer = DataWriter::CreateDataWriter(&buffer)?;
        data_writer.WriteBytes(&bgra)?;
        data_writer.DetachBuffer()?;
    }

    let software_bitmap = SoftwareBitmap::CreateCopyFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        width,
        height,
        BitmapAlphaMode::Premultiplied,
    )?;

    Ok(software_bitmap)
}

// ===== Shared Matching Logic =====

#[cfg(any(target_os = "macos", target_os = "windows", test))]
#[derive(Clone, Copy, PartialEq)]
enum MatchKind {
    Exact,
    Substring,
}

/// Trim surrounding whitespace and lowercase, so matching is case-insensitive
/// (PTCG Live uses mixed casing across locales).
#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

/// True when `text` is one of the needles. Matching is whole-string
/// (case-insensitive) so a stray fragment that happens to appear inside a
/// needle — e.g. "LO" inside "Battle Log" — cannot be mistaken for the button.
#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn matches_needle(text: &str, needles: &[String]) -> bool {
    let norm = normalize(text);
    needles.iter().any(|n| *n == norm)
}

/// True when `text` *contains* one of the needles as a substring, ignoring
/// punctuation and whitespace. Used for the transient toast, which OCR often
/// merges with neighbouring text (e.g. "cdierkens Export Copiedlondro") and
/// misreads the trailing "!" as a letter. Safe here because the needle is a
/// distinctive multi-word phrase.
#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn matches_substring(text: &str, needles: &[String]) -> bool {
    let hay = alphanumeric(text);
    needles.iter().any(|n| hay.contains(&alphanumeric(n)))
}

/// Lowercase and keep only alphanumeric characters, dropping punctuation and
/// whitespace so OCR noise (e.g. "!" read as "l") does not break a match.
#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn alphanumeric(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn needles() -> Vec<String> {
        vec![normalize("Battle Log"), normalize("Export"), normalize("Exportar")]
    }

    #[test]
    fn matches_exact_whole_string() {
        assert!(matches_needle("BATTLE LOG", &needles()));
        assert!(matches_needle("Export", &needles()));
        assert!(matches_needle("Exportar", &needles()));
    }

    #[test]
    fn matches_with_surrounding_whitespace() {
        assert!(matches_needle("  Battle Log  ", &needles()));
    }

    #[test]
    fn rejects_fragment_inside_a_needle() {
        assert!(!matches_needle("LO", &needles()));
        assert!(!matches_needle("Log", &needles()));
        assert!(!matches_needle("Expor", &needles()));
    }

    #[test]
    fn rejects_unrelated_text() {
        assert!(!matches_needle("INSPECT BOARD", &needles()));
        assert!(!matches_needle("Pokemon TCG Live", &needles()));
        assert!(!matches_needle("", &needles()));
    }

    #[test]
    fn substring_matches_merged_toast() {
        let toast = vec![normalize("Export Copied!")];
        assert!(matches_substring("cdierkens Export Copiedlondro", &toast));
        assert!(matches_substring("Export Copiedlondro", &toast));
        assert!(matches_substring("Export Copied!", &toast));
    }

    #[test]
    fn substring_rejects_when_phrase_absent() {
        let toast = vec![normalize("Export Copied!")];
        assert!(!matches_substring("BATTLE LOG", &toast));
        assert!(!matches_substring("cdierkens CONTINUE", &toast));
    }
}
