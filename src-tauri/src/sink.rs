//! Validates and saves a battle log pasted into the capture tab.

use std::sync::Mutex;

use serde::{Serialize, Serializer};

use crate::dedupe::SeenHashes;
use crate::gate::looks_like_battle_log;
use crate::ports::LogStore;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SubmitOutcome {
    Saved { path: String },
    AlreadySaved,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SubmitError {
    #[error("That doesn't look like a PTCG Live battle log.")]
    NotABattleLog,
    #[error("Couldn't save the log: {0}")]
    Store(String),
}

// Tauri hands command errors to the UI; send the display string.
impl Serialize for SubmitError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub trait LogSink: Send + Sync {
    fn submit(&self, text: &str) -> Result<SubmitOutcome, SubmitError>;
    /// The log's file was trashed; let the same text save again.
    fn forget(&self, text: &str);
}

/// Saves through a `LogStore`, skipping logs already saved this session.
pub struct StoreLogSink<S: LogStore> {
    store: S,
    seen: Mutex<SeenHashes>,
}

impl<S: LogStore> StoreLogSink<S> {
    pub fn new(store: S) -> Self {
        Self { store, seen: Mutex::new(SeenHashes::new()) }
    }
}

impl<S: LogStore + Send + Sync> LogSink for StoreLogSink<S> {
    fn submit(&self, text: &str) -> Result<SubmitOutcome, SubmitError> {
        let key = text.trim();
        if !looks_like_battle_log(key) {
            return Err(SubmitError::NotABattleLog);
        }
        let mut seen = self.seen.lock().map_err(|e| SubmitError::Store(e.to_string()))?;
        if !seen.is_new(key) {
            return Ok(SubmitOutcome::AlreadySaved);
        }
        let path = self.store.save(text).map_err(SubmitError::Store)?;
        seen.insert(key);
        Ok(SubmitOutcome::Saved { path })
    }

    fn forget(&self, text: &str) {
        if let Ok(mut seen) = self.seen.lock() {
            seen.forget(text.trim());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "Setup\ncdierkens drew 7 cards for the opening hand.\n- 7 drawn cards.\n\ncdierkens's Turn\ncdierkens drew a card.\n";

    #[derive(Default)]
    struct MemStore {
        saved: Mutex<Vec<String>>,
        fail: bool,
    }

    impl LogStore for MemStore {
        fn save(&self, text: &str) -> Result<String, String> {
            if self.fail {
                return Err("disk full".into());
            }
            let mut saved = self.saved.lock().unwrap();
            saved.push(text.to_string());
            Ok(format!("/logs/{}.txt", saved.len()))
        }
    }

    #[test]
    fn rejects_text_that_is_not_a_battle_log() {
        let sink = StoreLogSink::new(MemStore::default());
        assert_eq!(sink.submit("hello"), Err(SubmitError::NotABattleLog));
    }

    #[test]
    fn saves_a_new_log() {
        let sink = StoreLogSink::new(MemStore::default());
        assert_eq!(sink.submit(LOG), Ok(SubmitOutcome::Saved { path: "/logs/1.txt".into() }));
    }

    #[test]
    fn a_forgotten_log_saves_again() {
        let sink = StoreLogSink::new(MemStore::default());
        sink.submit(LOG).unwrap();
        sink.forget(&format!("\n{LOG}  "));
        assert!(matches!(sink.submit(LOG), Ok(SubmitOutcome::Saved { .. })));
        assert_eq!(sink.store.saved.lock().unwrap().len(), 2);
    }

    #[test]
    fn a_repeat_paste_is_already_saved() {
        let sink = StoreLogSink::new(MemStore::default());
        sink.submit(LOG).unwrap();
        assert_eq!(sink.submit(&format!("  {LOG}\n\n")), Ok(SubmitOutcome::AlreadySaved));
        assert_eq!(sink.store.saved.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_failed_save_can_be_retried() {
        let failing = StoreLogSink::new(MemStore { fail: true, ..MemStore::default() });
        assert_eq!(failing.submit(LOG), Err(SubmitError::Store("disk full".into())));
        assert!(failing.seen.lock().unwrap().is_new(LOG.trim()));
    }

    #[test]
    fn serializes_for_the_ui() {
        assert_eq!(
            serde_json::to_value(SubmitOutcome::Saved { path: "/x".into() }).unwrap(),
            serde_json::json!({ "kind": "saved", "path": "/x" })
        );
        assert_eq!(
            serde_json::to_value(SubmitOutcome::AlreadySaved).unwrap(),
            serde_json::json!({ "kind": "alreadySaved" })
        );
        assert_eq!(
            serde_json::to_value(SubmitError::NotABattleLog).unwrap(),
            serde_json::json!("That doesn't look like a PTCG Live battle log.")
        );
    }
}
