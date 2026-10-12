//! Persistent queue of deliveries: one saved log synced to one destination.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension};

/// Transient failures stop retrying this long after the first one.
const GIVE_UP_AFTER: Duration = Duration::from_secs(24 * 60 * 60);
const FIRST_RETRY: Duration = Duration::from_secs(30);
const MAX_RETRY: Duration = Duration::from_secs(60 * 60);
const SCHEMA_VERSION: i32 = 1;
const FILE_NAME: &str = "sync.sqlite";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryState {
    /// Saved from the results screen; waits until Undo is no longer possible.
    Held,
    Pending,
    Synced,
    Failed,
}

impl DeliveryState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Held => "held",
            Self::Pending => "pending",
            Self::Synced => "synced",
            Self::Failed => "failed",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        [Self::Held, Self::Pending, Self::Synced, Self::Failed].into_iter().find(|state| state.as_str() == s)
    }
}

/// When a newly queued delivery may be sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    Now,
    /// Starts `Held`.
    AfterResults,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    /// The saved log's file name in the log folder.
    pub log: String,
    pub destination: String,
    pub state: DeliveryState,
    /// Failed sends so far.
    pub attempts: u32,
    /// Unix milliseconds; only meaningful while `Pending`.
    pub next_attempt_at: i64,
    pub last_error: Option<String>,
}

/// A destination paused because it rejected its token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attention {
    pub destination: String,
    pub reason: String,
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("delivery queue: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("delivery queue folder: {0}")]
    Io(#[from] std::io::Error),
}

/// Delay before retry number `attempts` (1-based): 30s doubling, capped at 1h.
fn backoff(attempts: u32) -> Duration {
    let doublings = attempts.saturating_sub(1).min(16);
    FIRST_RETRY.saturating_mul(1 << doublings).min(MAX_RETRY)
}

/// Shared by commands and the delivery worker; `Connection` isn't `Sync`.
pub type SharedQueue = Arc<Mutex<DeliveryQueue>>;

/// A poisoned lock still holds a usable connection.
pub fn lock(queue: &SharedQueue) -> MutexGuard<'_, DeliveryQueue> {
    queue.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Times are Unix milliseconds, passed in so callers and tests own the clock.
pub struct DeliveryQueue {
    conn: Connection,
}

impl DeliveryQueue {
    /// `<app data>/sync.sqlite`. App state stays out of the log folder (ADR 0002).
    pub fn resolve_path(app: &tauri::AppHandle) -> tauri::Result<PathBuf> {
        use tauri::Manager;
        Ok(app.path().app_data_dir()?.join(FILE_NAME))
    }

    pub fn open(path: &Path) -> Result<Self, QueueError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::init(Connection::open(path)?)
    }

    /// Also the fallback when the file can't be opened, so saving logs never depends on sync.
    pub fn open_in_memory() -> Result<Self, QueueError> {
        Self::init(Connection::open_in_memory()?)
    }

    // Every table is `IF NOT EXISTS`, so this is safe on any database at this version.
    // A later schema change bumps `SCHEMA_VERSION` and adds a migration step here.
    fn init(conn: Connection) -> Result<Self, QueueError> {
        let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version <= SCHEMA_VERSION {
            conn.execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS deliveries (
                    log TEXT NOT NULL,
                    destination TEXT NOT NULL,
                    state TEXT NOT NULL,
                    attempts INTEGER NOT NULL DEFAULT 0,
                    first_failed_at INTEGER,
                    next_attempt_at INTEGER NOT NULL DEFAULT 0,
                    last_error TEXT,
                    PRIMARY KEY (log, destination)
                );
                CREATE TABLE IF NOT EXISTS destination_attention (
                    destination TEXT PRIMARY KEY,
                    reason TEXT NOT NULL
                );
                PRAGMA user_version = {SCHEMA_VERSION};"
            ))?;
        }
        Ok(Self { conn })
    }

    /// Queue `log` for `destination`, restarting any earlier delivery of it.
    pub fn enqueue(&self, log: &str, destination: &str, release: Release) -> Result<(), QueueError> {
        let state = match release {
            Release::Now => DeliveryState::Pending,
            Release::AfterResults => DeliveryState::Held,
        };
        self.conn.execute(
            "INSERT INTO deliveries (log, destination, state) VALUES (?1, ?2, ?3)
             ON CONFLICT (log, destination) DO UPDATE SET
                state = excluded.state, attempts = 0, first_failed_at = NULL,
                next_attempt_at = 0, last_error = NULL",
            params![log, destination, state.as_str()],
        )?;
        Ok(())
    }

    /// The results screen went away: `log` can no longer be undone.
    pub fn release_held(&self, log: &str) -> Result<(), QueueError> {
        self.conn.execute(
            "UPDATE deliveries SET state = ?2 WHERE log = ?1 AND state = ?3",
            params![log, DeliveryState::Pending.as_str(), DeliveryState::Held.as_str()],
        )?;
        Ok(())
    }

    /// At launch: no results screen is up, so nothing can be undone.
    pub fn release_all_held(&self) -> Result<(), QueueError> {
        self.conn.execute(
            "UPDATE deliveries SET state = ?1 WHERE state = ?2",
            params![DeliveryState::Pending.as_str(), DeliveryState::Held.as_str()],
        )?;
        Ok(())
    }

    /// Undo: drop deliveries of `log` that never left the app.
    pub fn drop_held(&self, log: &str) -> Result<(), QueueError> {
        self.conn.execute(
            "DELETE FROM deliveries WHERE log = ?1 AND state = ?2",
            params![log, DeliveryState::Held.as_str()],
        )?;
        Ok(())
    }

    /// The saved log was deleted locally; nothing is deleted remotely.
    pub fn forget_log(&self, log: &str) -> Result<(), QueueError> {
        self.conn.execute("DELETE FROM deliveries WHERE log = ?1", [log])?;
        Ok(())
    }

    pub fn remove_destination(&self, destination: &str) -> Result<(), QueueError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM deliveries WHERE destination = ?1", [destination])?;
        tx.execute("DELETE FROM destination_attention WHERE destination = ?1", [destination])?;
        tx.commit()?;
        Ok(())
    }

    /// The destination rejected its token: pause it until the player fixes it.
    pub fn flag_attention(&self, destination: &str, reason: &str) -> Result<(), QueueError> {
        self.conn.execute(
            "INSERT INTO destination_attention (destination, reason) VALUES (?1, ?2)
             ON CONFLICT (destination) DO UPDATE SET reason = excluded.reason",
            params![destination, reason],
        )?;
        Ok(())
    }

    /// A new token was saved: resume the destination's paused deliveries, due at once,
    /// with a fresh give-up window so the pause doesn't count against them.
    pub fn clear_attention(&self, destination: &str) -> Result<(), QueueError> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM destination_attention WHERE destination = ?1", [destination])?;
        tx.execute(
            "UPDATE deliveries SET first_failed_at = NULL, next_attempt_at = 0
             WHERE destination = ?1 AND state = ?2",
            params![destination, DeliveryState::Pending.as_str()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// For the drawer badge.
    pub fn needing_attention(&self) -> Result<Vec<Attention>, QueueError> {
        let mut stmt =
            self.conn.prepare("SELECT destination, reason FROM destination_attention ORDER BY destination")?;
        let rows = stmt.query_map([], |r| Ok(Attention { destination: r.get(0)?, reason: r.get(1)? }))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Pending deliveries whose next attempt is due, oldest first. Paused destinations are skipped.
    pub fn due(&self, now: i64) -> Result<Vec<Delivery>, QueueError> {
        self.select(
            "WHERE state = ?1 AND next_attempt_at <= ?2
               AND destination NOT IN (SELECT destination FROM destination_attention)
             ORDER BY next_attempt_at, rowid",
            params![DeliveryState::Pending.as_str(), now],
        )
    }

    /// Every delivery, for the match list's sync status.
    pub fn all(&self) -> Result<Vec<Delivery>, QueueError> {
        self.select("ORDER BY log, destination", [])
    }

    pub fn for_log(&self, log: &str) -> Result<Vec<Delivery>, QueueError> {
        self.select("WHERE log = ?1 ORDER BY destination", params![log])
    }

    // The `mark_*` results apply only to a delivery that is still pending, so a send that
    // finishes after Undo or a re-enqueue can't overwrite the newer state.

    pub fn mark_synced(&self, log: &str, destination: &str) -> Result<(), QueueError> {
        self.conn.execute(
            "UPDATE deliveries SET state = ?3, last_error = NULL
             WHERE log = ?1 AND destination = ?2 AND state = ?4",
            params![log, destination, DeliveryState::Synced.as_str(), DeliveryState::Pending.as_str()],
        )?;
        Ok(())
    }

    /// A transient failure: retry with backoff until `GIVE_UP_AFTER`, then fail.
    pub fn mark_retry(&self, log: &str, destination: &str, error: &str, now: i64) -> Result<(), QueueError> {
        let row: Option<(u32, Option<i64>)> = self
            .conn
            .query_row(
                "SELECT attempts, first_failed_at FROM deliveries
                 WHERE log = ?1 AND destination = ?2 AND state = ?3",
                params![log, destination, DeliveryState::Pending.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((attempts, first_failed_at)) = row else { return Ok(()) };
        let attempts = attempts + 1;
        let first = first_failed_at.unwrap_or(now);
        let gave_up = now.saturating_sub(first) >= GIVE_UP_AFTER.as_millis() as i64;
        let state = if gave_up { DeliveryState::Failed } else { DeliveryState::Pending };
        let next = now.saturating_add(backoff(attempts).as_millis() as i64);
        self.conn.execute(
            "UPDATE deliveries SET state = ?3, attempts = ?4, first_failed_at = ?5,
                next_attempt_at = ?6, last_error = ?7
             WHERE log = ?1 AND destination = ?2 AND state = ?8",
            params![log, destination, state.as_str(), attempts, first, next, error, DeliveryState::Pending.as_str()],
        )?;
        Ok(())
    }

    /// The destination rejected this log (a 4xx other than auth): no more retries.
    pub fn mark_failed(&self, log: &str, destination: &str, error: &str) -> Result<(), QueueError> {
        self.conn.execute(
            "UPDATE deliveries SET state = ?3, attempts = attempts + 1, last_error = ?4
             WHERE log = ?1 AND destination = ?2 AND state = ?5",
            params![log, destination, DeliveryState::Failed.as_str(), error, DeliveryState::Pending.as_str()],
        )?;
        Ok(())
    }

    fn select(&self, clause: &str, params: impl rusqlite::Params) -> Result<Vec<Delivery>, QueueError> {
        let sql =
            format!("SELECT log, destination, state, attempts, next_attempt_at, last_error FROM deliveries {clause}");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params, |r| {
            let state: String = r.get(2)?;
            Ok(Delivery {
                log: r.get(0)?,
                destination: r.get(1)?,
                state: DeliveryState::parse(&state).ok_or_else(|| {
                    rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, state.into())
                })?,
                attempts: r.get(3)?,
                next_attempt_at: r.get(4)?,
                last_error: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use DeliveryState::*;

    const HOUR: i64 = 60 * 60 * 1000;

    fn queue() -> DeliveryQueue {
        DeliveryQueue::open_in_memory().unwrap()
    }

    fn state(q: &DeliveryQueue, log: &str, dest: &str) -> Option<DeliveryState> {
        q.for_log(log).unwrap().into_iter().find(|d| d.destination == dest).map(|d| d.state)
    }

    #[test]
    fn backoff_doubles_from_30s_and_caps_at_an_hour() {
        assert_eq!(backoff(1), Duration::from_secs(30));
        assert_eq!(backoff(2), Duration::from_secs(60));
        assert_eq!(backoff(3), Duration::from_secs(120));
        assert_eq!(backoff(20), MAX_RETRY);
        assert_eq!(backoff(u32::MAX), MAX_RETRY);
    }

    #[test]
    fn pending_deliveries_are_due_at_once() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", Release::Now).unwrap();
        let due = q.due(0).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].log, "a.txt");
        assert_eq!(due[0].state, Pending);
    }

    #[test]
    fn held_deliveries_wait_until_released() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", Release::AfterResults).unwrap();
        assert!(q.due(0).unwrap().is_empty());
        q.release_held("a.txt").unwrap();
        assert_eq!(q.due(0).unwrap().len(), 1);
    }

    #[test]
    fn undo_drops_only_held_deliveries() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", Release::AfterResults).unwrap();
        q.enqueue("a.txt", "court", Release::Now).unwrap();
        q.drop_held("a.txt").unwrap();
        assert_eq!(state(&q, "a.txt", "pokedojo"), None);
        assert_eq!(state(&q, "a.txt", "court"), Some(Pending));
    }

    #[test]
    fn transient_failures_back_off() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", Release::Now).unwrap();
        q.mark_retry("a.txt", "pokedojo", "timeout", 1_000).unwrap();
        assert!(q.due(1_000).unwrap().is_empty());
        let due = q.due(1_000 + 30_000).unwrap();
        assert_eq!(due[0].attempts, 1);
        assert_eq!(due[0].next_attempt_at, 31_000);
        assert_eq!(due[0].last_error.as_deref(), Some("timeout"));
    }

    #[test]
    fn transient_failures_give_up_after_a_day() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", Release::Now).unwrap();
        q.mark_retry("a.txt", "pokedojo", "503", 0).unwrap();
        q.mark_retry("a.txt", "pokedojo", "503", 23 * HOUR).unwrap();
        assert_eq!(state(&q, "a.txt", "pokedojo"), Some(Pending));
        q.mark_retry("a.txt", "pokedojo", "503", 24 * HOUR).unwrap();
        assert_eq!(state(&q, "a.txt", "pokedojo"), Some(Failed));
        assert!(q.due(i64::MAX).unwrap().is_empty());
    }

    #[test]
    fn permanent_failures_stop_at_once() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", Release::Now).unwrap();
        q.mark_failed("a.txt", "pokedojo", "422").unwrap();
        assert_eq!(state(&q, "a.txt", "pokedojo"), Some(Failed));
        assert!(q.due(i64::MAX).unwrap().is_empty());
    }

    #[test]
    fn flagged_destinations_pause_until_cleared() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", Release::Now).unwrap();
        q.enqueue("a.txt", "court", Release::Now).unwrap();
        q.flag_attention("pokedojo", "401 Unauthorized").unwrap();
        let due: Vec<_> = q.due(0).unwrap().into_iter().map(|d| d.destination).collect();
        assert_eq!(due, ["court"]);
        assert_eq!(state(&q, "a.txt", "pokedojo"), Some(Pending));
        assert_eq!(
            q.needing_attention().unwrap(),
            [Attention { destination: "pokedojo".to_owned(), reason: "401 Unauthorized".to_owned() }]
        );
        q.clear_attention("pokedojo").unwrap();
        assert_eq!(q.due(0).unwrap().len(), 2);
        assert!(q.needing_attention().unwrap().is_empty());
    }

    #[test]
    fn removing_a_destination_clears_its_flag() {
        let q = queue();
        q.flag_attention("pokedojo", "403").unwrap();
        q.remove_destination("pokedojo").unwrap();
        assert!(q.needing_attention().unwrap().is_empty());
    }

    #[test]
    fn synced_deliveries_leave_the_queue() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", Release::Now).unwrap();
        q.mark_synced("a.txt", "pokedojo").unwrap();
        assert_eq!(state(&q, "a.txt", "pokedojo"), Some(Synced));
        assert!(q.due(i64::MAX).unwrap().is_empty());
    }

    #[test]
    fn enqueueing_again_restarts_a_failed_delivery() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", Release::Now).unwrap();
        q.mark_retry("a.txt", "pokedojo", "503", 0).unwrap();
        q.mark_failed("a.txt", "pokedojo", "422").unwrap();
        q.enqueue("a.txt", "pokedojo", Release::Now).unwrap();
        let due = q.due(0).unwrap();
        assert_eq!(due[0].attempts, 0);
        assert_eq!(due[0].last_error, None);
    }

    #[test]
    fn deleting_a_log_or_destination_forgets_its_deliveries() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", Release::Now).unwrap();
        q.enqueue("b.txt", "pokedojo", Release::Now).unwrap();
        q.enqueue("b.txt", "court", Release::Now).unwrap();
        q.forget_log("a.txt").unwrap();
        assert!(q.for_log("a.txt").unwrap().is_empty());
        q.remove_destination("pokedojo").unwrap();
        let left = q.for_log("b.txt").unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].destination, "court");
    }

    #[test]
    fn all_lists_every_delivery_for_the_match_list() {
        let q = queue();
        q.enqueue("b.txt", "pokedojo", Release::AfterResults).unwrap();
        q.enqueue("a.txt", "pokedojo", Release::Now).unwrap();
        q.mark_synced("a.txt", "pokedojo").unwrap();
        let all: Vec<_> = q.all().unwrap().into_iter().map(|d| (d.log, d.state)).collect();
        assert_eq!(all, [("a.txt".to_owned(), Synced), ("b.txt".to_owned(), Held)]);
    }

    #[test]
    fn a_pause_does_not_count_toward_giving_up() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", Release::Now).unwrap();
        q.mark_retry("a.txt", "pokedojo", "503", 0).unwrap();
        q.flag_attention("pokedojo", "401").unwrap();
        q.clear_attention("pokedojo").unwrap();
        let due = q.due(1).unwrap();
        assert_eq!(due.len(), 1, "due at once after the token fix");
        q.mark_retry("a.txt", "pokedojo", "503", 48 * HOUR).unwrap();
        assert_eq!(state(&q, "a.txt", "pokedojo"), Some(Pending));
    }

    #[test]
    fn a_late_send_result_cannot_release_a_re_held_delivery() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", Release::Now).unwrap();
        q.enqueue("a.txt", "pokedojo", Release::AfterResults).unwrap();
        q.mark_retry("a.txt", "pokedojo", "timeout", 0).unwrap();
        q.mark_synced("a.txt", "pokedojo").unwrap();
        q.mark_failed("a.txt", "pokedojo", "422").unwrap();
        let d = &q.for_log("a.txt").unwrap()[0];
        assert_eq!((d.state, d.attempts), (Held, 0));
    }

    #[test]
    fn launch_releases_every_held_delivery() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", Release::AfterResults).unwrap();
        q.enqueue("b.txt", "pokedojo", Release::AfterResults).unwrap();
        q.release_all_held().unwrap();
        assert_eq!(q.due(0).unwrap().len(), 2);
    }

    #[test]
    fn a_database_from_before_the_attention_table_gains_it() {
        let dir = std::env::temp_dir().join(format!("ptcgl-queue-old-{}", std::process::id()));
        let path = dir.join("sync.sqlite");
        std::fs::create_dir_all(&dir).unwrap();
        Connection::open(&path)
            .unwrap()
            .execute_batch(
                "CREATE TABLE deliveries (
                    log TEXT NOT NULL, destination TEXT NOT NULL, state TEXT NOT NULL,
                    attempts INTEGER NOT NULL DEFAULT 0, first_failed_at INTEGER,
                    next_attempt_at INTEGER NOT NULL DEFAULT 0, last_error TEXT,
                    PRIMARY KEY (log, destination));
                 PRAGMA user_version = 1;",
            )
            .unwrap();
        let q = DeliveryQueue::open(&path).unwrap();
        q.flag_attention("pokedojo", "401").unwrap();
        assert!(q.due(0).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_queue_survives_reopening() {
        let dir = std::env::temp_dir().join(format!("ptcgl-queue-{}", std::process::id()));
        let path = dir.join("sync.sqlite");
        {
            let q = DeliveryQueue::open(&path).unwrap();
            q.enqueue("a.txt", "pokedojo", Release::Now).unwrap();
        }
        let q = DeliveryQueue::open(&path).unwrap();
        assert_eq!(q.due(0).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
