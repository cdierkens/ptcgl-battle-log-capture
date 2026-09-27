use unicode_normalization::UnicodeNormalization;

/// Canonical game-window title hint. "Pokemon TCG Live" (ASCII) on some
/// platforms, "Pokémon TCG Live" (accented) on others — normalize diacritics
/// so a single hint matches both.
pub const GAME_TITLE_HINT: &str = "pokemon";

/// Case- and accent-insensitive match of a window title against a hint.
pub fn matches_title(title: &str, hint: &str) -> bool {
    normalize(title).contains(&normalize(hint))
}

/// Lowercase and strip combining diacritic marks (NFD decomposition) so
/// "Pokémon" and "Pokemon" compare equal.
fn normalize(s: &str) -> String {
    s.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_accented_title_with_ascii_hint() {
        assert!(matches_title("Pokémon TCG Live", "pokemon"));
    }

    #[test]
    fn matches_ascii_title_with_ascii_hint() {
        assert!(matches_title("Pokemon TCG Live", "pokemon"));
    }

    #[test]
    fn rejects_unrelated_title() {
        assert!(!matches_title("Google Chrome", "pokemon"));
    }

    #[test]
    fn normalize_strips_diacritics_and_lowercases() {
        assert_eq!(normalize("Pokémon TCG Live"), "pokemon tcg live");
    }
}