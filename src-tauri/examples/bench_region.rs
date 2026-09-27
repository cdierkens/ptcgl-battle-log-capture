// Compare: full-frame OCR vs OCR on just the region where the Battle Log tab
// sits on the results screen (fixed-size window => fixed position).
use std::time::Instant;
use battle_log_capture_lib::platform::frame::XcapFrameCapturer;
use battle_log_capture_lib::platform::window::XcapWindowProvider;
use battle_log_capture_lib::ports::{FrameCapturer, WindowProvider, Frame, TextRecognizer};
use battle_log_capture_lib::platform::ocr::VisionTextRecognizer;
use battle_log_capture_lib::buttons::battle_log_needles;

fn crop(f: &Frame, x0: f32, y0: f32, x1: f32, y1: f32) -> Frame {
    let fx = |v: f32, max: u32| ((v * max as f32).round() as i32).clamp(0, max as i32) as u32;
    let x0 = fx(x0, f.width); let x1 = fx(x1, f.width);
    let y0 = fx(y0, f.height); let y1 = fx(y1, f.height);
    let w = x1 - x0; let h = y1 - y0;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in y0..y1 {
        let start = ((y * f.width + x0) * 4) as usize;
        let len = (w * 4) as usize;
        rgba.extend_from_slice(&f.rgba[start..start + len]);
    }
    Frame { width: w, height: h, rgba }
}

fn main() {
    let wp = XcapWindowProvider::new("pokemon");
    let Some(window) = wp.find_game_window() else { println!("no window"); return };
    let fc = XcapFrameCapturer;
    let Some(frame) = fc.capture(&window) else { println!("no frame"); return };
    let ocr = VisionTextRecognizer;
    let needles = battle_log_needles();

    // region around where BATTLE LOG appears on the results screen (measured)
    let region = crop(&frame, 0.40, 0.76, 0.60, 0.86);
    println!("region size: {}x{}", region.width, region.height);

    // full frame
    let mut t1 = vec![];
    for _ in 0..5 { let t = Instant::now(); let _ = ocr.find_text(&frame, &needles); t1.push(t.elapsed().as_secs_f64()*1000.0); }
    t1.sort_by(|a,b| a.partial_cmp(b).unwrap());
    println!("full-frame ocr:    median {:.1} ms", t1[2]);

    // region only
    let mut t2 = vec![];
    for _ in 0..5 { let t = Instant::now(); let _ = ocr.find_text(&region, &needles); t2.push(t.elapsed().as_secs_f64()*1000.0); }
    t2.sort_by(|a,b| a.partial_cmp(b).unwrap());
    println!("region-only ocr:   median {:.1} ms", t2[2]);
    println!("region match: {:?}", ocr.find_text(&region, &needles).len());
    println!("full   match: {:?}", ocr.find_text(&frame, &needles).len());
}
