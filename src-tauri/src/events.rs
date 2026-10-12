use serde::Serialize;

use crate::overlay_state::OverlayState;

/// An overlay state change, emitted to the main window and logged.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatus {
    pub state: String,
    pub message: String,
    pub detail: Option<String>,
    pub ts: u64,
}

impl CaptureStatus {
    pub fn new(state: OverlayState, message: impl Into<String>, detail: Option<String>) -> Self {
        Self { state: state.name().to_string(), message: message.into(), detail, ts: now_millis() }
    }
}

fn now_millis() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_camel_case() {
        let status =
            CaptureStatus::new(OverlayState::InMatch, "Watching for the results screen", Some("detail".to_string()));
        let json = serde_json::to_value(status).unwrap();
        assert_eq!(json["state"], "InMatch");
        assert_eq!(json["message"], "Watching for the results screen");
        assert_eq!(json["detail"], "detail");
        assert!(json["ts"].is_u64());
    }

    #[test]
    fn detail_may_be_none() {
        let status = CaptureStatus::new(OverlayState::NoWindow, "Waiting", None);
        let json = serde_json::to_value(status).unwrap();
        assert!(json["detail"].is_null());
    }
}
