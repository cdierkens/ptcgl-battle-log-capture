/// Localized results-screen labels, extracted from PTCG Live's translation
/// cache (`localization-cache/{monthly}/{locale}`). Matching is
/// case-insensitive, so the mixed casing across locales is fine.
use crate::coords::RelativeRect;

pub struct ButtonLabels {
    pub battle_log: &'static str,
    pub continue_label: &'static str,
}

pub const BUTTON_LABELS: &[(&str, ButtonLabels)] = &[
    ("en",    ButtonLabels { battle_log: "Battle Log",           continue_label: "Continue" }),
    ("de",    ButtonLabels { battle_log: "Kampf-Protokoll",      continue_label: "Fortfahren" }),
    ("es",    ButtonLabels { battle_log: "registro del combate", continue_label: "Continuar" }),
    ("es_la", ButtonLabels { battle_log: "registro del combate", continue_label: "Continuar" }),
    ("fr",    ButtonLabels { battle_log: "Journal de combat",    continue_label: "Continuer" }),
    ("it",    ButtonLabels { battle_log: "registro lotta",       continue_label: "Continua" }),
    ("ptbr",  ButtonLabels { battle_log: "Registro de Batalha",  continue_label: "Continuar" }),
];

/// Needle set for the "Battle Log" label across all locales: the results-screen
/// tab (earliest reliable match-over signal) and the open drawer's title.
pub fn battle_log_needles() -> Vec<String> {
    BUTTON_LABELS.iter().map(|(_, l)| l.battle_log.to_string()).collect()
}

/// Needle set for the results screen's "Continue" button across all locales;
/// the Continue shield is placed over the first match.
pub fn continue_needles() -> Vec<String> {
    BUTTON_LABELS.iter().map(|(_, l)| l.continue_label.to_string()).collect()
}

/// Region of the results screen where the "Battle Log" tab label renders, as
/// fractions of the captured frame. Measured from the live client across three
/// separate matches: the tab's top-left sat at x=1220, y=1213 in the
/// 2560x1496 capture every time (only its width varied 119..127px with the
/// localized label length). This rect pads that box with margin while staying
/// ~1.5% of the frame, so recognizing it is ~4x cheaper than the full window.
pub fn battle_log_region() -> RelativeRect {
    RelativeRect { x0: 0.43, y0: 0.76, x1: 0.58, y1: 0.86 }
}

/// Region of the results screen where the Continue button label renders, as
/// fractions of the captured frame. Measured live: "CONTINUE" at x 0.470–0.531,
/// y 0.939–0.961 of a 2560x1496 capture; padded so localized labels still fit.
pub fn continue_region() -> RelativeRect {
    RelativeRect { x0: 0.40, y0: 0.90, x1: 0.60, y1: 1.0 }
}

/// Region at the top of the window where the open Battle Log drawer shows its
/// own "Battle Log" title. Seeing it keeps the results screen "present" while
/// the drawer covers the results-screen tab.
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
    fn continue_needles_cover_english_and_german() {
        let needles = continue_needles();
        assert!(needles.iter().any(|n| n == "Continue"));
        assert!(needles.iter().any(|n| n == "Fortfahren"));
    }

    #[test]
    fn every_locale_has_all_labels() {
        for (locale, labels) in BUTTON_LABELS {
            for (field, value) in [("battle_log", labels.battle_log), ("continue", labels.continue_label)] {
                assert!(!value.trim().is_empty(), "{locale} {field} must be non-empty");
            }
        }
    }
}
