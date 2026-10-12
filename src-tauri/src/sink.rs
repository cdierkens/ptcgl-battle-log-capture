//! Validates and saves a battle log pasted into the capture tab.

use std::sync::Mutex;

use serde::{Serialize, Serializer};

use crate::dedupe::SeenHashes;
use crate::delivery_queue::{self, Release, SharedQueue};
use crate::gate::looks_like_battle_log;
use crate::logs;
use crate::ports::LogStore;
use crate::settings::{self, SharedSettings};

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

/// Queues a delivery to each enabled destination after every new save. A save made while the
/// results screen is up is held until it goes away, so Undo never races the network.
pub struct SyncingLogSink<S: LogSink> {
    inner: S,
    settings: SharedSettings,
    queue: SharedQueue,
    results_up: Box<dyn Fn() -> bool + Send + Sync>,
}

impl<S: LogSink> SyncingLogSink<S> {
    pub fn new(
        inner: S,
        settings: SharedSettings,
        queue: SharedQueue,
        results_up: Box<dyn Fn() -> bool + Send + Sync>,
    ) -> Self {
        Self { inner, settings, queue, results_up }
    }

    /// Sync failures never fail the save; the log is already on disk.
    fn enqueue(&self, path: &str) {
        let file_name = match logs::file_name_of(path) {
            Ok(name) => name,
            Err(e) => return tracing::warn!("sync: {e}"),
        };
        let release = if (self.results_up)() { Release::AfterResults } else { Release::Now };
        let queue = delivery_queue::lock(&self.queue);
        for destination in settings::current(&self.settings).destinations.iter().filter(|d| d.enabled) {
            if let Err(e) = queue.enqueue(&file_name, &destination.id, release) {
                tracing::warn!("sync: couldn't queue {file_name} for {}: {e}", destination.name);
            }
        }
    }
}

impl<S: LogSink> LogSink for SyncingLogSink<S> {
    fn submit(&self, text: &str) -> Result<SubmitOutcome, SubmitError> {
        let outcome = self.inner.submit(text)?;
        if let SubmitOutcome::Saved { path } = &outcome {
            self.enqueue(path);
        }
        Ok(outcome)
    }

    fn forget(&self, text: &str) {
        self.inner.forget(text);
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

    mod syncing {
        use std::collections::BTreeMap;
        use std::path::Path;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::{Arc, Mutex, RwLock};

        use super::*;
        use crate::delivery_queue::{DeliveryQueue, DeliveryState};
        use crate::destinations::Destination;
        use crate::settings::Settings;

        fn destination(id: &str, enabled: bool) -> Destination {
            Destination {
                id: id.into(),
                name: id.into(),
                url: format!("https://{id}.example/"),
                headers: BTreeMap::new(),
                token_url: None,
                enabled,
            }
        }

        fn rig(results_up: bool) -> (SyncingLogSink<StoreLogSink<MemStore>>, SharedQueue, Arc<AtomicBool>) {
            let settings = Settings {
                destinations: vec![destination("on", true), destination("off", false)],
                ..Settings::defaults(Path::new("/logs"))
            };
            let queue: SharedQueue = Arc::new(Mutex::new(DeliveryQueue::open_in_memory().unwrap()));
            let up = Arc::new(AtomicBool::new(results_up));
            let flag = Arc::clone(&up);
            let sink = SyncingLogSink::new(
                StoreLogSink::new(MemStore::default()),
                Arc::new(RwLock::new(settings)),
                Arc::clone(&queue),
                Box::new(move || flag.load(Ordering::SeqCst)),
            );
            (sink, queue, up)
        }

        #[test]
        fn a_save_queues_each_enabled_destination() {
            let (sink, queue, _) = rig(false);
            sink.submit(LOG).unwrap();
            let all = delivery_queue::lock(&queue).all().unwrap();
            assert_eq!(all.len(), 1);
            assert_eq!((all[0].log.as_str(), all[0].destination.as_str()), ("1.txt", "on"));
            assert_eq!(all[0].state, DeliveryState::Pending);
        }

        #[test]
        fn a_save_on_the_results_screen_is_held() {
            let (sink, queue, _) = rig(true);
            sink.submit(LOG).unwrap();
            assert_eq!(delivery_queue::lock(&queue).all().unwrap()[0].state, DeliveryState::Held);
        }

        #[test]
        fn a_repeat_or_rejected_paste_queues_nothing() {
            let (sink, queue, _) = rig(false);
            sink.submit(LOG).unwrap();
            sink.submit(LOG).unwrap();
            let _ = sink.submit("hello");
            assert_eq!(delivery_queue::lock(&queue).all().unwrap().len(), 1);
        }
    }
}
