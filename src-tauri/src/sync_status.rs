//! What the UI shows about sync: each delivery's state, and the destinations needing attention.

use serde::Serialize;

use crate::delivery_queue::{Attention, DeliveryQueue, DeliveryState, QueueError};

/// Emitted when a delivery or a destination's attention changes; the payload is empty.
pub const SYNC_CHANGED_EVENT: &str = "sync://changed";

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub deliveries: Vec<DeliveryStatus>,
    pub attention: Vec<Attention>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryStatus {
    /// The saved log's file name.
    pub log: String,
    pub destination: String,
    pub state: SyncState,
    pub last_error: Option<String>,
}

/// Held reads as pending: the player only cares that it hasn't gone yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncState {
    Pending,
    Synced,
    Failed,
}

impl From<DeliveryState> for SyncState {
    fn from(state: DeliveryState) -> Self {
        match state {
            DeliveryState::Held | DeliveryState::Pending => Self::Pending,
            DeliveryState::Synced => Self::Synced,
            DeliveryState::Failed => Self::Failed,
        }
    }
}

pub fn read(queue: &DeliveryQueue) -> Result<SyncStatus, QueueError> {
    let deliveries = queue
        .all()?
        .into_iter()
        .map(|d| DeliveryStatus {
            log: d.log,
            destination: d.destination,
            state: d.state.into(),
            last_error: d.last_error,
        })
        .collect();
    Ok(SyncStatus { deliveries, attention: queue.needing_attention()? })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delivery_queue::Release;

    #[test]
    fn held_reads_as_pending_and_errors_carry_through() {
        let q = DeliveryQueue::open_in_memory().unwrap();
        q.enqueue("a.txt", "d", Release::AfterResults).unwrap();
        q.enqueue("b.txt", "d", Release::Now).unwrap();
        q.mark_failed("b.txt", "d", "HTTP 422").unwrap();
        q.flag_attention("d", "rejected").unwrap();
        let status = read(&q).unwrap();
        let states: Vec<_> =
            status.deliveries.iter().map(|d| (d.log.as_str(), d.state, d.last_error.as_deref())).collect();
        assert_eq!(states, [("a.txt", SyncState::Pending, None), ("b.txt", SyncState::Failed, Some("HTTP 422"))]);
        assert_eq!(status.attention, [Attention { destination: "d".into(), reason: "rejected".into() }]);
    }

    #[test]
    fn serializes_for_the_ui() {
        let status = SyncStatus {
            deliveries: vec![DeliveryStatus {
                log: "a.txt".into(),
                destination: "d".into(),
                state: SyncState::Synced,
                last_error: None,
            }],
            attention: vec![Attention { destination: "d".into(), reason: "r".into() }],
        };
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            serde_json::json!({
                "deliveries": [{"log": "a.txt", "destination": "d", "state": "synced", "lastError": null}],
                "attention": [{"destination": "d", "reason": "r"}],
            })
        );
    }
}
