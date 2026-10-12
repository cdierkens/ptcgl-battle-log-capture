//! Delivery worker: releases held deliveries once the results screen is gone, then sends
//! whatever is due as an envelope POST.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use tracing::warn;

use crate::delivery_queue::{self, Delivery, SharedQueue};
use crate::destinations::Destination;
use crate::logs;
use crate::ports::{DeliveryRequest, DeliveryTransport, SecretStore};
use crate::settings::{self, Settings, SharedSettings};
use crate::sink::ResultsUp;

/// Also how soon a held delivery is released after the results screen goes away.
const TICK: Duration = Duration::from_secs(1);
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<'a> {
    schema_version: u32,
    log: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    player_name: Option<&'a str>,
    saved_at: String,
    app_version: &'a str,
}

/// What a send means for its delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    Synced,
    Retry(String),
    /// The site rejected the token; pause the destination.
    NeedsAttention(String),
    Failed(String),
}

fn classify(response: Result<u16, String>) -> Outcome {
    match response {
        Err(e) => Outcome::Retry(e),
        Ok(200..=299) => Outcome::Synced,
        Ok(status @ (401 | 403)) => Outcome::NeedsAttention(format!("The site rejected the token (HTTP {status}).")),
        Ok(status @ (429 | 500..=599)) => Outcome::Retry(format!("HTTP {status}")),
        Ok(status) => Outcome::Failed(format!("HTTP {status}")),
    }
}

pub struct Worker {
    pub queue: SharedQueue,
    pub settings: SharedSettings,
    pub secrets: Arc<dyn SecretStore>,
    pub transport: Box<dyn DeliveryTransport>,
    /// Held deliveries wait while this is true (Undo is still possible).
    pub results_up: ResultsUp,
    pub app_version: String,
}

impl Worker {
    pub fn run(self) -> ! {
        loop {
            self.tick(now_ms());
            std::thread::sleep(TICK);
        }
    }

    pub fn tick(&self, now: i64) {
        let due = {
            // Check under the queue lock so a save can't be held between the check and the release.
            let queue = delivery_queue::lock(&self.queue);
            if !(self.results_up)() {
                log_err(queue.release_all_held());
            }
            match queue.due(now) {
                Ok(due) => due,
                Err(e) => return warn!("sync: couldn't read the queue: {e}"),
            }
        };
        let mut tokens = HashMap::new();
        let mut paused = HashSet::new();
        for delivery in due {
            if paused.contains(&delivery.destination) {
                continue;
            }
            if let Some(Outcome::NeedsAttention(_)) = self.deliver(&delivery, &mut tokens, now) {
                paused.insert(delivery.destination);
            }
        }
    }

    /// Sends one delivery and records the result. `None` when nothing was sent.
    /// Settings are read per delivery, since a slow tick can outlive a change to them.
    fn deliver(&self, delivery: &Delivery, tokens: &mut Tokens, now: i64) -> Option<Outcome> {
        let queue = || delivery_queue::lock(&self.queue);
        let (log, dest_id) = (delivery.log.as_str(), delivery.destination.as_str());
        // Every due row was queued for a destination already in settings, so a missing one was removed.
        let settings = settings::current(&self.settings);
        let Some(destination) = settings.destinations.iter().find(|d| d.id == dest_id) else {
            log_err(queue().remove_destination(dest_id));
            return None;
        };
        if !destination.enabled {
            return None;
        }
        let token =
            tokens.entry(dest_id.to_string()).or_insert_with(|| self.secrets.get(dest_id).map_err(|e| e.to_string()));
        let outcome = match token {
            Ok(Some(token)) => self.send(&settings, destination, token, log)?,
            Ok(None) => Outcome::NeedsAttention("No token is saved for this destination.".into()),
            Err(e) => Outcome::Retry(e.clone()),
        };
        let q = queue();
        log_err(match &outcome {
            Outcome::Synced => q.mark_synced(log, dest_id),
            Outcome::Retry(e) => q.mark_retry(log, dest_id, e, now),
            Outcome::NeedsAttention(reason) => q.flag_attention(dest_id, reason),
            Outcome::Failed(e) => q.mark_failed(log, dest_id, e),
        });
        Some(outcome)
    }

    /// `None` when the saved log was deleted: its deliveries are dropped, nothing is sent.
    fn send(&self, settings: &Settings, destination: &Destination, token: &str, log: &str) -> Option<Outcome> {
        let dir = Path::new(&settings.output_dir);
        let path = dir.join(log);
        if !path.exists() {
            log_err(delivery_queue::lock(&self.queue).forget_log(log));
            return None;
        }
        let text = match logs::read_log(dir, log) {
            Ok(text) => text,
            Err(e) => return Some(Outcome::Retry(e)),
        };
        let envelope = Envelope {
            schema_version: SCHEMA_VERSION,
            log: &text,
            player_name: Some(settings.player_name.trim()).filter(|name| !name.is_empty()),
            saved_at: saved_at(&path),
            app_version: &self.app_version,
        };
        let body = match serde_json::to_string(&envelope) {
            Ok(body) => body,
            Err(e) => return Some(Outcome::Failed(e.to_string())),
        };
        let request = DeliveryRequest { url: &destination.url, token, headers: &destination.headers, body: &body };
        Some(classify(self.transport.post(&request)))
    }
}

/// Each destination's token, read from the keychain at most once per tick.
type Tokens = HashMap<String, Result<Option<String>, String>>;

fn log_err<E: std::fmt::Display>(result: Result<(), E>) {
    if let Err(e) = result {
        warn!("sync: {e}");
    }
}

/// When the saved log was last written, RFC 3339 in UTC.
fn saved_at(path: &Path) -> String {
    let time = std::fs::metadata(path).and_then(|m| m.modified()).unwrap_or_else(|_| SystemTime::now());
    DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Mutex, RwLock};

    use super::*;
    use crate::delivery_queue::{DeliveryQueue, DeliveryState, Release};
    use crate::ports::MemSecretStore;

    const LOG: &str = "log.txt";
    const DEST: &str = "dest-1";

    /// url, token, body, headers
    type Sent = (String, String, String, BTreeMap<String, String>);

    #[derive(Default)]
    struct FakeTransport {
        responses: Mutex<Vec<Result<u16, String>>>,
        sent: Mutex<Vec<Sent>>,
    }

    impl DeliveryTransport for Arc<FakeTransport> {
        fn post(&self, r: &DeliveryRequest<'_>) -> Result<u16, String> {
            self.sent.lock().unwrap().push((r.url.into(), r.token.into(), r.body.into(), r.headers.clone()));
            self.responses.lock().unwrap().pop().unwrap_or(Ok(200))
        }
    }

    struct Rig {
        worker: Worker,
        transport: Arc<FakeTransport>,
        results_up: Arc<AtomicBool>,
        dir: PathBuf,
    }

    impl Drop for Rig {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    impl Rig {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("ptcgl-sync-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(LOG), "Setup\nash drew 7 cards.\n").unwrap();
            let destination = Destination {
                id: DEST.into(),
                name: "Dojo".into(),
                url: "https://dojo.example/api".into(),
                headers: BTreeMap::from([("X-Client".into(), "tracker".into())]),
                token_url: None,
                enabled: true,
            };
            let settings =
                Settings { player_name: " ash ".into(), destinations: vec![destination], ..Settings::defaults(&dir) };
            let secrets = MemSecretStore::default();
            secrets.set(DEST, "tok").unwrap();
            let transport = Arc::new(FakeTransport::default());
            let results_up = Arc::new(AtomicBool::new(false));
            let up = Arc::clone(&results_up);
            let worker = Worker {
                queue: Arc::new(Mutex::new(DeliveryQueue::open_in_memory().unwrap())),
                settings: Arc::new(RwLock::new(settings)),
                secrets: Arc::new(secrets),
                transport: Box::new(Arc::clone(&transport)),
                results_up: Box::new(move || up.load(Ordering::SeqCst)),
                app_version: "1.2.3".into(),
            };
            Self { worker, transport, results_up, dir }
        }

        fn respond(&self, response: Result<u16, String>) {
            self.transport.responses.lock().unwrap().push(response);
        }

        fn enqueue(&self, release: Release) {
            delivery_queue::lock(&self.worker.queue).enqueue(LOG, DEST, release).unwrap();
        }

        fn state(&self) -> Option<DeliveryState> {
            delivery_queue::lock(&self.worker.queue).for_log(LOG).unwrap().first().map(|d| d.state)
        }

        fn sent(&self) -> usize {
            self.transport.sent.lock().unwrap().len()
        }

        fn update_settings(&self, f: impl FnOnce(&mut Settings)) {
            let mut s = settings::current(&self.worker.settings);
            f(&mut s);
            settings::replace(&self.worker.settings, s);
        }
    }

    #[test]
    fn classifies_responses_per_the_spec() {
        assert_eq!(classify(Ok(201)), Outcome::Synced);
        assert!(matches!(classify(Err("offline".into())), Outcome::Retry(_)));
        assert!(matches!(classify(Ok(429)), Outcome::Retry(_)));
        assert!(matches!(classify(Ok(503)), Outcome::Retry(_)));
        assert!(matches!(classify(Ok(401)), Outcome::NeedsAttention(_)));
        assert!(matches!(classify(Ok(403)), Outcome::NeedsAttention(_)));
        assert!(matches!(classify(Ok(400)), Outcome::Failed(_)));
        assert!(matches!(classify(Ok(302)), Outcome::Failed(_)));
    }

    #[test]
    fn sends_the_envelope_with_auth_and_static_headers() {
        let rig = Rig::new("envelope");
        rig.enqueue(Release::Now);
        rig.worker.tick(0);
        assert_eq!(rig.state(), Some(DeliveryState::Synced));
        let sent = rig.transport.sent.lock().unwrap();
        let (url, token, body, headers) = &sent[0];
        assert_eq!(url, "https://dojo.example/api");
        assert_eq!(token, "tok");
        assert_eq!(headers.get("X-Client").map(String::as_str), Some("tracker"));
        let json: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(json["schemaVersion"], 1);
        assert_eq!(json["log"], "Setup\nash drew 7 cards.\n");
        assert_eq!(json["playerName"], "ash");
        assert_eq!(json["appVersion"], "1.2.3");
        assert!(DateTime::parse_from_rfc3339(json["savedAt"].as_str().unwrap()).is_ok());
    }

    #[test]
    fn player_name_is_left_out_when_unset() {
        let rig = Rig::new("no-name");
        rig.update_settings(|s| s.player_name = "  ".into());
        rig.enqueue(Release::Now);
        rig.worker.tick(0);
        let body = rig.transport.sent.lock().unwrap()[0].2.clone();
        assert!(!body.contains("playerName"));
    }

    #[test]
    fn held_deliveries_wait_for_the_results_screen_to_go() {
        let rig = Rig::new("held");
        rig.results_up.store(true, Ordering::SeqCst);
        rig.enqueue(Release::AfterResults);
        rig.worker.tick(0);
        assert_eq!(rig.sent(), 0);
        assert_eq!(rig.state(), Some(DeliveryState::Held));
        rig.results_up.store(false, Ordering::SeqCst);
        rig.worker.tick(0);
        assert_eq!(rig.state(), Some(DeliveryState::Synced));
    }

    #[test]
    fn transient_failures_retry_later() {
        let rig = Rig::new("retry");
        rig.respond(Ok(503));
        rig.enqueue(Release::Now);
        rig.worker.tick(0);
        assert_eq!(rig.state(), Some(DeliveryState::Pending));
        rig.worker.tick(1_000);
        assert_eq!(rig.sent(), 1, "not due until the backoff passes");
        rig.worker.tick(60_000);
        assert_eq!(rig.state(), Some(DeliveryState::Synced));
    }

    #[test]
    fn a_rejected_token_pauses_the_destination_after_one_request() {
        let rig = Rig::new("auth");
        delivery_queue::lock(&rig.worker.queue).enqueue("other.txt", DEST, Release::Now).unwrap();
        fs::write(rig.dir.join("other.txt"), "x").unwrap();
        rig.respond(Ok(401));
        rig.enqueue(Release::Now);
        rig.worker.tick(0);
        assert_eq!(rig.sent(), 1, "the second delivery isn't tried with a bad token");
        let queue = delivery_queue::lock(&rig.worker.queue);
        assert_eq!(queue.needing_attention().unwrap().len(), 1);
        assert!(queue.all().unwrap().iter().all(|d| d.state == DeliveryState::Pending));
    }

    #[test]
    fn other_client_errors_fail_that_delivery() {
        let rig = Rig::new("bad-request");
        rig.respond(Ok(422));
        rig.enqueue(Release::Now);
        rig.worker.tick(0);
        assert_eq!(rig.state(), Some(DeliveryState::Failed));
    }

    #[test]
    fn a_missing_token_needs_attention_without_sending() {
        let rig = Rig::new("no-token");
        rig.worker.secrets.delete(DEST).unwrap();
        rig.enqueue(Release::Now);
        rig.worker.tick(0);
        assert_eq!(rig.sent(), 0);
        assert_eq!(delivery_queue::lock(&rig.worker.queue).needing_attention().unwrap().len(), 1);
    }

    #[test]
    fn disabled_destinations_keep_their_deliveries_pending() {
        let rig = Rig::new("disabled");
        rig.update_settings(|s| s.destinations[0].enabled = false);
        rig.enqueue(Release::Now);
        rig.worker.tick(0);
        assert_eq!(rig.sent(), 0);
        assert_eq!(rig.state(), Some(DeliveryState::Pending));
    }

    #[test]
    fn a_deleted_log_is_dropped_not_sent() {
        let rig = Rig::new("deleted");
        fs::remove_file(rig.dir.join(LOG)).unwrap();
        rig.enqueue(Release::Now);
        rig.worker.tick(0);
        assert_eq!(rig.sent(), 0);
        assert_eq!(rig.state(), None);
    }

    #[test]
    fn deliveries_for_a_removed_destination_are_dropped() {
        let rig = Rig::new("removed");
        rig.update_settings(|s| s.destinations.clear());
        rig.enqueue(Release::Now);
        rig.worker.tick(0);
        assert_eq!(rig.sent(), 0);
        assert_eq!(rig.state(), None);
    }

    fn add_second_log(rig: &Rig) {
        fs::write(rig.dir.join("two.txt"), "Setup\n").unwrap();
        let queue = delivery_queue::lock(&rig.worker.queue);
        queue.enqueue(LOG, DEST, Release::Now).unwrap();
        queue.enqueue("two.txt", DEST, Release::Now).unwrap();
    }

    struct CountingSecrets(MemSecretStore, Arc<std::sync::atomic::AtomicUsize>);

    impl SecretStore for CountingSecrets {
        fn get(&self, account: &str) -> Result<Option<String>, crate::ports::SecretError> {
            self.1.fetch_add(1, Ordering::SeqCst);
            self.0.get(account)
        }
        fn set(&self, account: &str, secret: &str) -> Result<(), crate::ports::SecretError> {
            self.0.set(account, secret)
        }
        fn delete(&self, account: &str) -> Result<(), crate::ports::SecretError> {
            self.0.delete(account)
        }
    }

    #[test]
    fn reads_each_token_once_per_tick() {
        let mut rig = Rig::new("token-once");
        let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let secrets = MemSecretStore::default();
        secrets.set(DEST, "tok").unwrap();
        rig.worker.secrets = Arc::new(CountingSecrets(secrets, Arc::clone(&reads)));
        add_second_log(&rig);
        rig.worker.tick(0);
        assert_eq!(rig.sent(), 2);
        assert_eq!(reads.load(Ordering::SeqCst), 1);
    }

    struct DisablingTransport(SharedSettings, Arc<FakeTransport>);

    impl DeliveryTransport for DisablingTransport {
        fn post(&self, r: &DeliveryRequest<'_>) -> Result<u16, String> {
            let mut s = settings::current(&self.0);
            s.destinations[0].enabled = false;
            settings::replace(&self.0, s);
            self.1.post(r)
        }
    }

    #[test]
    fn a_destination_disabled_mid_tick_gets_no_more_sends() {
        let mut rig = Rig::new("disable-mid-tick");
        rig.worker.transport =
            Box::new(DisablingTransport(Arc::clone(&rig.worker.settings), Arc::clone(&rig.transport)));
        add_second_log(&rig);
        rig.worker.tick(0);
        assert_eq!(rig.sent(), 1);
    }
}
