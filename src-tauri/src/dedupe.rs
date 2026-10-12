use std::collections::HashSet;

use sha2::{Digest, Sha256};

/// Tracks which battle-log texts have already been seen, by SHA-256 hash, so
/// re-clicking the same results screen never writes a duplicate file.
#[derive(Debug, Default)]
pub struct SeenHashes {
    seen: HashSet<[u8; 32]>,
}

impl SeenHashes {
    pub fn new() -> Self {
        Self { seen: HashSet::new() }
    }

    /// True if `text` has not been recorded yet.
    pub fn is_new(&self, text: &str) -> bool {
        !self.seen.contains(&hash(text))
    }

    /// Record `text`, returning true if it was not already present.
    pub fn insert(&mut self, text: &str) -> bool {
        self.seen.insert(hash(text))
    }

    /// Drop `text` after Undo/Delete so the same log can be saved again. True if it was present.
    pub fn forget(&mut self, text: &str) -> bool {
        self.seen.remove(&hash(text))
    }
}

fn hash(text: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_text_is_new() {
        let seen = SeenHashes::new();
        assert!(seen.is_new("Setup\ncdierkens's Turn\n"));
    }

    #[test]
    fn forget_makes_text_new_again() {
        let mut seen = SeenHashes::new();
        seen.insert("log one");
        assert!(seen.forget("log one"));
        assert!(seen.is_new("log one"));
        assert!(!seen.forget("log one"));
    }

    #[test]
    fn insert_then_not_new() {
        let mut seen = SeenHashes::new();
        assert!(seen.insert("log one"));
        assert!(!seen.is_new("log one"));
        assert!(!seen.insert("log one"));
    }

    #[test]
    fn distinct_texts_are_distinct() {
        let mut seen = SeenHashes::new();
        assert!(seen.insert("log one"));
        assert!(seen.is_new("log two"));
    }
}
