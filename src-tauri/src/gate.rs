/// Coarse "does this clipboard text look like a battle log?" gate. The Rust
/// side only accepts text whose first non-blank line is a recognized phase
/// header AND which contains at least one "- " sub-entry line. The authoritative
/// validator is the TypeScript parser in @dierkens.dev/ptcgl-battle-log-parser; this just avoids
/// saving arbitrary clipboard content.
pub fn looks_like_battle_log(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|line| !line.is_empty()).collect();

    if lines.is_empty() {
        return false;
    }

    let first = lines[0];
    let has_sub_entry = lines.iter().any(|line| line.starts_with("- "));
    let header_ok = first == "Setup"
        || first.ends_with("'s Turn")
        || first == "Pokémon Checkup"
        || first == "Preparación"
        || first == "Vorbereitung"
        || first == "Préparation";

    header_ok && has_sub_entry
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_LOG: &str = "Setup\ncdierkens drew 7 cards for the opening hand.\n- 7 drawn cards.\n\ncdierkens's Turn\ncdierkens drew a card.\ncdierkens ended their turn.\n";

    #[test]
    fn accepts_a_real_battle_log() {
        assert!(looks_like_battle_log(SAMPLE_LOG));
    }

    #[test]
    fn rejects_empty_text() {
        assert!(!looks_like_battle_log(""));
    }

    #[test]
    fn rejects_text_without_sub_entries() {
        assert!(!looks_like_battle_log("Setup\njust one line\n"));
    }

    #[test]
    fn rejects_text_with_wrong_header() {
        assert!(!looks_like_battle_log("hello\n- world\n"));
    }
}
