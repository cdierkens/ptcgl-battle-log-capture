//! Persistent queue of deliveries: one saved log synced to one destination.

use std::path::Path;
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension};

/// Transient failures stop retrying this long after the first one.
pub const GIVE_UP_AFTER: Duration = Duration::from_secs(24 * 60 * 60);
const FIRST_RETRY: Duration = Duration::from_secs(30);
const MAX_RETRY: Duration = Duration::from_secs(60 * 60);
const SCHEMA_VERSION: i32 = 1;

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

    fn parse(s: &str) -> rusqlite::Result<Self> {
        match s {
            "held" => Ok(Self::Held),
            "pending" => Ok(Self::Pending),
            "synced" => Ok(Self::Synced),
            "failed" => Ok(Self::Failed),
            other => Err(rusqlite::Error::InvalidColumnType(0, other.to_owned(), rusqlite::types::Type::Text)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    /// The saved log's file name in the log folder.
    pub log: String,
    pub destination: String,
    pub state: DeliveryState,
    pub attempts: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("delivery queue: {0}")]
pub struct QueueError(#[from] rusqlite::Error);

/// Delay before retry number `attempts` (1-based): 30s doubling, capped at 1h.
pub fn backoff(attempts: u32) -> Duration {
    let doublings = attempts.saturating_sub(1).min(16);
    FIRST_RETRY.saturating_mul(1 << doublings).min(MAX_RETRY)
}

/// Times are Unix milliseconds, passed in so callers and tests own the clock.
pub struct DeliveryQueue {
    conn: Connection,
}

impl DeliveryQueue {
    pub fn open(path: &Path) -> Result<Self, QueueError> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self, QueueError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, QueueError> {
        let version: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < SCHEMA_VERSION {
            conn.execute_batch(
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
                PRAGMA user_version = 1;",
            )?;
        }
        Ok(Self { conn })
    }

    /// Queue `log` for `destination`, restarting any earlier delivery of it.
    pub fn enqueue(&self, log: &str, destination: &str, held: bool) -> Result<(), QueueError> {
        let state = if held { DeliveryState::Held } else { DeliveryState::Pending };
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
        self.conn.execute("UPDATE deliveries SET state = 'pending' WHERE log = ?1 AND state = 'held'", [log])?;
        Ok(())
    }

    /// Undo: drop deliveries of `log` that never left the app.
    pub fn drop_held(&self, log: &str) -> Result<(), QueueError> {
        self.conn.execute("DELETE FROM deliveries WHERE log = ?1 AND state = 'held'", [log])?;
        Ok(())
    }

    /// The saved log was deleted locally; nothing is deleted remotely.
    pub fn forget_log(&self, log: &str) -> Result<(), QueueError> {
        self.conn.execute("DELETE FROM deliveries WHERE log = ?1", [log])?;
        Ok(())
    }

    pub fn remove_destination(&self, destination: &str) -> Result<(), QueueError> {
        self.conn.execute("DELETE FROM deliveries WHERE destination = ?1", [destination])?;
        Ok(())
    }

    /// Pending deliveries whose next attempt is due, oldest first.
    pub fn due(&self, now: i64) -> Result<Vec<Delivery>, QueueError> {
        self.select("WHERE state = 'pending' AND next_attempt_at <= ?1 ORDER BY next_attempt_at, rowid", params![now])
    }

    pub fn for_log(&self, log: &str) -> Result<Vec<Delivery>, QueueError> {
        self.select("WHERE log = ?1 ORDER BY destination", params![log])
    }

    pub fn mark_synced(&self, log: &str, destination: &str) -> Result<(), QueueError> {
        self.conn.execute(
            "UPDATE deliveries SET state = 'synced', last_error = NULL WHERE log = ?1 AND destination = ?2",
            params![log, destination],
        )?;
        Ok(())
    }

    /// A transient failure: retry with backoff until `GIVE_UP_AFTER`, then fail.
    pub fn mark_retry(&self, log: &str, destination: &str, error: &str, now: i64) -> Result<(), QueueError> {
        let row: Option<(u32, Option<i64>)> = self
            .conn
            .query_row(
                "SELECT attempts, first_failed_at FROM deliveries WHERE log = ?1 AND destination = ?2",
                params![log, destination],
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
             WHERE log = ?1 AND destination = ?2",
            params![log, destination, state.as_str(), attempts, first, next, error],
        )?;
        Ok(())
    }

    /// A permanent failure (bad token, rejected request): no more retries.
    pub fn mark_failed(&self, log: &str, destination: &str, error: &str) -> Result<(), QueueError> {
        self.conn.execute(
            "UPDATE deliveries SET state = 'failed', attempts = attempts + 1, last_error = ?3
             WHERE log = ?1 AND destination = ?2",
            params![log, destination, error],
        )?;
        Ok(())
    }

    fn select(&self, clause: &str, params: impl rusqlite::Params) -> Result<Vec<Delivery>, QueueError> {
        let sql = format!("SELECT log, destination, state, attempts, last_error FROM deliveries {clause}");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params, |r| {
            Ok(Delivery {
                log: r.get(0)?,
                destination: r.get(1)?,
                state: DeliveryState::parse(&r.get::<_, String>(2)?)?,
                attempts: r.get(3)?,
                last_error: r.get(4)?,
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
        q.enqueue("a.txt", "pokedojo", false).unwrap();
        let due = q.due(0).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].log, "a.txt");
        assert_eq!(due[0].state, Pending);
    }

    #[test]
    fn held_deliveries_wait_until_released() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", true).unwrap();
        assert!(q.due(0).unwrap().is_empty());
        q.release_held("a.txt").unwrap();
        assert_eq!(q.due(0).unwrap().len(), 1);
    }

    #[test]
    fn undo_drops_only_held_deliveries() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", true).unwrap();
        q.enqueue("a.txt", "court", false).unwrap();
        q.drop_held("a.txt").unwrap();
        assert_eq!(state(&q, "a.txt", "pokedojo"), None);
        assert_eq!(state(&q, "a.txt", "court"), Some(Pending));
    }

    #[test]
    fn transient_failures_back_off() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", false).unwrap();
        q.mark_retry("a.txt", "pokedojo", "timeout", 1_000).unwrap();
        assert!(q.due(1_000).unwrap().is_empty());
        let due = q.due(1_000 + 30_000).unwrap();
        assert_eq!(due[0].attempts, 1);
        assert_eq!(due[0].last_error.as_deref(), Some("timeout"));
    }

    #[test]
    fn transient_failures_give_up_after_a_day() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", false).unwrap();
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
        q.enqueue("a.txt", "pokedojo", false).unwrap();
        q.mark_failed("a.txt", "pokedojo", "401").unwrap();
        assert_eq!(state(&q, "a.txt", "pokedojo"), Some(Failed));
        assert!(q.due(i64::MAX).unwrap().is_empty());
    }

    #[test]
    fn synced_deliveries_leave_the_queue() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", false).unwrap();
        q.mark_synced("a.txt", "pokedojo").unwrap();
        assert_eq!(state(&q, "a.txt", "pokedojo"), Some(Synced));
        assert!(q.due(i64::MAX).unwrap().is_empty());
    }

    #[test]
    fn enqueueing_again_restarts_a_failed_delivery() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", false).unwrap();
        q.mark_retry("a.txt", "pokedojo", "503", 0).unwrap();
        q.mark_failed("a.txt", "pokedojo", "401").unwrap();
        q.enqueue("a.txt", "pokedojo", false).unwrap();
        let due = q.due(0).unwrap();
        assert_eq!(due[0].attempts, 0);
        assert_eq!(due[0].last_error, None);
    }

    #[test]
    fn deleting_a_log_or_destination_forgets_its_deliveries() {
        let q = queue();
        q.enqueue("a.txt", "pokedojo", false).unwrap();
        q.enqueue("b.txt", "pokedojo", false).unwrap();
        q.enqueue("b.txt", "court", false).unwrap();
        q.forget_log("a.txt").unwrap();
        assert!(q.for_log("a.txt").unwrap().is_empty());
        q.remove_destination("pokedojo").unwrap();
        let left = q.for_log("b.txt").unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].destination, "court");
    }

    #[test]
    fn the_queue_survives_reopening() {
        let dir = std::env::temp_dir().join(format!("ptcgl-queue-{}", std::process::id()));
        let path = dir.join("sync.sqlite");
        {
            let q = DeliveryQueue::open(&path).unwrap();
            q.enqueue("a.txt", "pokedojo", false).unwrap();
        }
        let q = DeliveryQueue::open(&path).unwrap();
        assert_eq!(q.due(0).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
