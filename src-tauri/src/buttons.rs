/// Localized button labels for the capture sequence, extracted from PTCG Live's
/// translation cache (`localization-cache/{monthly}/{locale}`). Keys:
/// `blog_loc_battle_log`, `deck_export`, `global_confirm`, `global_close`,
/// `blog_loc_export_copied`. Matching is case-insensitive, so the mixed casing
/// observed in the source (some locales lowercase) is fine.
use crate::coords::RelativeRect;

pub struct ButtonLabels {
    pub battle_log: &'static str,
    pub export: &'static str,
    pub confirm: &'static str,
    pub close: &'static str,
    pub export_copied: &'static str,
    pub continue_label: &'static str,
}

pub const BUTTON_LABELS: &[(&str, ButtonLabels)] = &[
    ("en",    ButtonLabels { battle_log: "Battle Log",  export: "Export",     confirm: "Confirm",    close: "close",    export_copied: "Export Copied!",     continue_label: "Continue" }),
    ("de",    ButtonLabels { battle_log: "Kampf-Protokoll", export: "Exportieren", confirm: "Bestätigen", close: "Schließen", export_copied: "Export kopiert!", continue_label: "Fortfahren" }),
    ("es",    ButtonLabels { battle_log: "registro del combate", export: "Exportar", confirm: "confirmar", close: "cerrar", export_copied: "¡Información copiada!", continue_label: "Continuar" }),
    ("es_la", ButtonLabels { battle_log: "registro del combate", export: "Exportar", confirm: "confirmar", close: "cerrar", export_copied: "¡Información copiada!", continue_label: "Continuar" }),
    ("fr",    ButtonLabels { battle_log: "Journal de combat", export: "Exporter", confirm: "confirmer", close: "fermer", export_copied: "Export copié !", continue_label: "Continuer" }),
    ("it",    ButtonLabels { battle_log: "registro lotta", export: "Esporta", confirm: "conferma", close: "chiudi", export_copied: "Esportazione copiata!", continue_label: "Continua" }),
    ("ptbr",  ButtonLabels { battle_log: "Registro de Batalha", export: "Exportar", confirm: "confirmar", close: "fechar", export_copied: "Exportar copiado!", continue_label: "Continuar" }),
];

/// Needle set for the "Battle Log" (open-panel) button across all locales. This
/// tab is the earliest reliable signal that a match has ended — the reason lines
/// ("Opponent conceded", etc.) all render on the same results screen, so there
/// is no usable pre-results signal.
pub fn battle_log_needles() -> Vec<String> {
    BUTTON_LABELS.iter().map(|(_, l)| l.battle_log.to_string()).collect()
}

/// Needle set for the "Export" button across all locales.
pub fn export_needles() -> Vec<String> {
    BUTTON_LABELS.iter().map(|(_, l)| l.export.to_string()).collect()
}

/// Needle set for the "Confirm" dialog button across all locales.
pub fn confirm_needles() -> Vec<String> {
    BUTTON_LABELS.iter().map(|(_, l)| l.confirm.to_string()).collect()
}

/// Needle set for the "close" (dismiss) button across all locales.
pub fn close_needles() -> Vec<String> {
    BUTTON_LABELS.iter().map(|(_, l)| l.close.to_string()).collect()
}

/// Needle set for the transient "Export Copied!" toast, the game's own positive
/// confirmation that the battle log reached the clipboard.
pub fn export_copied_needles() -> Vec<String> {
    BUTTON_LABELS.iter().map(|(_, l)| l.export_copied.to_string()).collect()
}

/// Needle set for the "Continue" button across all locales. It is the results
/// screen's primary CTA — after a capture, its absence is the signal that the
/// player has taken back over (the results screen is dismissed), so the engine
/// can safely re-arm for the next match.
pub fn continue_needles() -> Vec<String> {
    BUTTON_LABELS.iter().map(|(_, l)| l.continue_label.to_string()).collect()
}

/// A UI element with no text label, located as a fraction of the game window.
/// The PTCG Live window is fixed-size, so these fractions are stable across
/// runs and device pixel densities (they are applied to the *window* size, not
/// the captured frame).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelativePoint {
    pub label: &'static str,
    pub x_frac: f32,
    pub y_frac: f32,
}

/// The Export and Close controls in the Battle Log drawer's top-right header are
/// icon-only (a document-with-arrow and a circle-X), so OCR cannot read them.
/// Their positions were measured from the live client: Export at ~0.962/0.066,
/// Close at ~0.987/0.065 of the 1280x748 window.
pub const DRAWER_ICONS: &[RelativePoint] = &[
    RelativePoint { label: "Export icon", x_frac: 0.9621, y_frac: 0.0662 },
    RelativePoint { label: "Close icon", x_frac: 0.9867, y_frac: 0.0648 },
];

/// The drawer Export icon, as a fraction of the game window.
pub fn export_icon() -> RelativePoint {
    DRAWER_ICONS[0]
}

/// The drawer Close icon, as a fraction of the game window.
pub fn close_icon() -> RelativePoint {
    DRAWER_ICONS[1]
}

/// A window-fraction box centered on the Export icon, used as the seed crop
/// region when discovering the Export-icon reference template. The icon is
/// icon-only, so OCR cannot find it; this box bounds its known header position
/// and whatever lands in it at discover time becomes the calibrated template.
pub fn export_icon_region() -> RelativeRect {
    let p = export_icon();
    RelativeRect {
        x0: (p.x_frac - 0.04).max(0.0),
        y0: (p.y_frac - 0.04).max(0.0),
        x1: (p.x_frac + 0.04).min(1.0),
        y1: (p.y_frac + 0.04).min(1.0),
    }
}

/// Search window for template-matching the Export icon at run time. Tight band
/// around the drawer's Export icon (x 0.90..1.0, y 0.01..0.15) so the NCC scan
/// stays cheap; the template is the anchor, this is only the search window.
pub fn export_icon_search_region() -> RelativeRect {
    RelativeRect { x0: 0.90, y0: 0.01, x1: 1.0, y1: 0.15 }
}

/// Region of the results screen where the "Battle Log" tab label renders, as
/// fractions of the captured frame. Measured from the live client across three
/// separate matches: the tab's top-left sat at x=1220, y=1213 in the
/// 2560x1496 capture every time (only its width varied 119..127px with the
/// localized label length). That is x=0.477, y=0.811, spanning to roughly
/// x=0.526, y=0.824. This rect pads that box with margin while staying ~1.5% of
/// the frame, so recognizing it is ~4x cheaper than the full window.
///
/// Only the exact "Battle Log" label appears in this strip, and matching is
/// whole-string, so ordinary board/hand text here cannot produce a false hit.
pub fn battle_log_region() -> RelativeRect {
    RelativeRect { x0: 0.43, y0: 0.76, x1: 0.58, y1: 0.86 }
}

/// Region at the top of the window where the Battle Log drawer's own "Battle Log"
/// title appears once the drawer is open. This is a *different* "BATTLE LOG"
/// label from the results-screen tab in `battle_log_region` (which sits at the
/// bottom): the tab is present when the drawer is closed and opens it, while
/// this title is only present while the drawer is open. Detecting it confirms
/// the drawer actually opened, replacing a fixed post-click wait.
pub fn drawer_title_region() -> RelativeRect {
    RelativeRect { x0: 0.0, y0: 0.0, x1: 1.0, y1: 0.25 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battle_log_needles_cover_english_and_german() {
        let needles = battle_log_needles();
        assert!(needles.iter().any(|n| n == "Battle Log"));
        assert!(needles.iter().any(|n| n == "Kampf-Protokoll"));
    }

    #[test]
    fn export_needles_cover_english_and_spanish() {
        let needles = export_needles();
        assert!(needles.iter().any(|n| n == "Export"));
        assert!(needles.iter().any(|n| n == "Exportar"));
    }

    #[test]
    fn confirm_needles_cover_english_and_german() {
        let needles = confirm_needles();
        assert!(needles.iter().any(|n| n == "Confirm"));
        assert!(needles.iter().any(|n| n == "Bestätigen"));
    }

    #[test]
    fn close_needles_cover_english_and_french() {
        let needles = close_needles();
        assert!(needles.iter().any(|n| n == "close"));
        assert!(needles.iter().any(|n| n == "fermer"));
    }

    #[test]
    fn export_copied_needles_cover_english_and_german() {
        let needles = export_copied_needles();
        assert!(needles.iter().any(|n| n == "Export Copied!"));
        assert!(needles.iter().any(|n| n == "Export kopiert!"));
    }

    #[test]
    fn continue_needles_cover_english_and_german() {
        let needles = continue_needles();
        assert!(needles.iter().any(|n| n == "Continue"));
        assert!(needles.iter().any(|n| n == "Fortfahren"));
    }

    #[test]
    fn every_locale_has_all_labels() {
        for (locale, labels) in BUTTON_LABELS {
            for (field, value) in [
                ("battle_log", labels.battle_log),
                ("export", labels.export),
                ("confirm", labels.confirm),
                ("close", labels.close),
                ("export_copied", labels.export_copied),
                ("continue", labels.continue_label),
            ] {
                assert!(!value.trim().is_empty(), "{locale} {field} must be non-empty");
            }
        }
    }

    #[test]
    fn export_icon_region_contains_the_export_icon() {
        let region = export_icon_region();
        let p = export_icon();
        assert!(region.x0 <= p.x_frac && p.x_frac <= region.x1);
        assert!(region.y0 <= p.y_frac && p.y_frac <= region.y1);
    }

    #[test]
    fn export_icon_search_region_contains_the_export_icon() {
        let region = export_icon_search_region();
        let p = export_icon();
        assert!(region.x0 <= p.x_frac && p.x_frac <= region.x1);
        assert!(region.y0 <= p.y_frac && p.y_frac <= region.y1);
    }
}
