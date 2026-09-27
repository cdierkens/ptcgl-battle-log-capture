use serde::Serialize;

use crate::state::CaptureState;

/// A capture-loop state transition, emitted to the frontend and logged.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatus {
    pub state: String,
    pub message: String,
    pub detail: Option<String>,
    pub ts: u64,
}

impl CaptureStatus {
    pub fn new(state: CaptureState, message: impl Into<String>, detail: Option<String>) -> Self {
        Self {
            state: state.to_string(),
            message: message.into(),
            detail,
            ts: now_millis(),
        }
    }

    /// A status emit that is not a state-machine transition (e.g. capture
    /// started/stopped), for events the UI needs but the engine state does not model.
    pub fn custom(state: &str, message: impl Into<String>, detail: Option<String>) -> Self {
        Self {
            state: state.to_string(),
            message: message.into(),
            detail,
            ts: now_millis(),
        }
    }
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_camel_case() {
        let status = CaptureStatus::new(CaptureState::LogTabDetected, "Results screen detected", Some("score=0.9842".to_string()));
        let json = serde_json::to_value(status).unwrap();
        assert_eq!(json["state"], "LogTabDetected");
        assert_eq!(json["message"], "Results screen detected");
        assert_eq!(json["detail"], "score=0.9842");
        assert!(json["ts"].is_u64());
    }

    #[test]
    fn detail_may_be_none() {
        let status = CaptureStatus::new(CaptureState::Idle, "Waiting", None);
        let json = serde_json::to_value(status).unwrap();
        assert!(json["detail"].is_null());
    }
}