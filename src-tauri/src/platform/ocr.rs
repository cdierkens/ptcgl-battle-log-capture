//! Text recognition for the capture engine.
//!
//! On macOS this is backed by the Vision framework (VNRecognizeTextRequest).
//! On every other platform it is a no-op stub that never matches, so the
//! engine compiles everywhere while only doing real work on macOS.

#[cfg(target_os = "macos")]
use std::io::Cursor;

#[cfg(target_os = "macos")]
use image::{ImageFormat, RgbaImage};
#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;
#[cfg(target_os = "macos")]
use objc2::AnyThread;
#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSData, NSDictionary};
#[cfg(target_os = "macos")]
use objc2_vision::{
    VNImageOption, VNImageRequestHandler, VNRecognizeTextRequest, VNRequest,
    VNRequestTextRecognitionLevel,
};

use crate::ports::{Frame, TextMatch, TextRecognizer};

/// Text recognizer backed by macOS Vision. On non-macOS platforms this
/// reports no matches.
pub struct VisionTextRecognizer;

#[cfg(target_os = "macos")]
impl TextRecognizer for VisionTextRecognizer {
    fn find_text(&self, frame: &Frame, needles: &[String]) -> Vec<TextMatch> {
        self.scan(frame, needles, MatchKind::Exact)
    }

    fn find_text_containing(&self, frame: &Frame, needles: &[String]) -> Vec<TextMatch> {
        self.scan(frame, needles, MatchKind::Substring)
    }
}

#[cfg(not(target_os = "macos"))]
impl TextRecognizer for VisionTextRecognizer {
    fn find_text(&self, _frame: &Frame, _needles: &[String]) -> Vec<TextMatch> {
        Vec::new()
    }

    fn find_text_containing(&self, _frame: &Frame, _needles: &[String]) -> Vec<TextMatch> {
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, PartialEq)]
enum MatchKind {
    /// Recognized text must equal a needle (whitespace-trimmed, lowercased).
    Exact,
    /// Recognized text must contain a needle as a substring.
    Substring,
}

#[cfg(target_os = "macos")]
impl VisionTextRecognizer {
    fn scan(&self, frame: &Frame, needles: &[String], kind: MatchKind) -> Vec<TextMatch> {
        // Encode the raw RGBA frame as PNG so Vision can decode it.
        let Some(rgba) = RgbaImage::from_raw(frame.width, frame.height, frame.rgba.clone()) else {
            return Vec::new();
        };
        let mut png = Vec::new();
        if rgba.write_to(&mut Cursor::new(&mut png), ImageFormat::Png).is_err() {
            return Vec::new();
        }

        let data = NSData::with_bytes(&png);
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
            // Bounding box is normalized with a bottom-left origin.
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
}

#[cfg(not(target_os = "macos"))]
impl TextRecognizer for VisionTextRecognizer {
    fn find_text(&self, _frame: &Frame, _needles: &[String]) -> Vec<TextMatch> {
        Vec::new()
    }
}

/// Trim surrounding whitespace and lowercase, so matching is case-insensitive
/// (PTCG Live uses mixed casing across locales).
#[cfg(target_os = "macos")]
fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

/// True when `text` is one of the needles. Matching is whole-string
/// (case-insensitive) so a stray fragment that happens to appear inside a
/// needle — e.g. "LO" inside "Battle Log" — cannot be mistaken for the button.
#[cfg(target_os = "macos")]
fn matches_needle(text: &str, needles: &[String]) -> bool {
    let norm = normalize(text);
    needles.iter().any(|n| *n == norm)
}

/// True when `text` *contains* one of the needles as a substring, ignoring
/// punctuation and whitespace. Used for the transient toast, which Vision often
/// merges with neighbouring text (e.g. "cdierkens Export Copiedlondro") and
/// misreads the trailing "!" as a letter. Safe here because the needle is a
/// distinctive multi-word phrase.
#[cfg(target_os = "macos")]
fn matches_substring(text: &str, needles: &[String]) -> bool {
    let hay = alphanumeric(text);
    needles.iter().any(|n| hay.contains(&alphanumeric(n)))
}

/// Lowercase and keep only alphanumeric characters, dropping punctuation and
/// whitespace so OCR noise (e.g. "!" read as "l") does not break a match.
#[cfg(target_os = "macos")]
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
        // "LO" is a substring of "battle log" but is not the whole label.
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
        // Vision merges the toast with adjacent labels and misreads "!" as "l".
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