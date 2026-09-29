//! Capture engine mode: `capture` (auto-export) or `discover` (calibrate).
//! Persisted independently of the user-editable settings so the settings form
//! cannot clobber it. Missing or corrupt state falls back to `Capture`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Capture,
    Discover,
}

/// Where the mode is persisted. Lives next to `settings.json` in the base dir.
pub fn mode_path() -> PathBuf {
    settings::base_dir().join("mode.json")
}

/// Load the persisted mode, defaulting to `Capture` when the file is missing
/// or unreadable.
pub fn load() -> Mode {
    load_from(&mode_path())
}

/// Persist the mode, creating the parent directory if needed.
pub fn save(mode: Mode) -> Result<(), String> {
    save_to(&mode_path(), mode)
}

fn load_from(path: &Path) -> Mode {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_to(path: &Path, mode: Mode) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&mode).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!("ptcgl-mode-{tag}-{nanos}"));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn load_defaults_to_capture_when_missing() {
        let dir = TempDir::new("missing");
        assert_eq!(load_from(&dir.path().join("mode.json")), Mode::Capture);
    }

    #[test]
    fn load_defaults_to_capture_when_corrupt() {
        let dir = TempDir::new("corrupt");
        let path = dir.path().join("mode.json");
        fs::write(&path, "not json").unwrap();
        assert_eq!(load_from(&path), Mode::Capture);
    }

    #[test]
    fn save_and_load_round_trips_discover() {
        let dir = TempDir::new("roundtrip");
        let path = dir.path().join("nested").join("mode.json");
        save_to(&path, Mode::Discover).unwrap();
        assert_eq!(load_from(&path), Mode::Discover);
    }

    #[test]
    fn save_and_load_round_trips_capture() {
        let dir = TempDir::new("capture");
        let path = dir.path().join("mode.json");
        save_to(&path, Mode::Capture).unwrap();
        assert_eq!(load_from(&path), Mode::Capture);
    }

    #[test]
    fn serialization_is_lowercase() {
        assert_eq!(serde_json::to_string(&Mode::Capture).unwrap(), "\"capture\"");
        assert_eq!(serde_json::to_string(&Mode::Discover).unwrap(), "\"discover\"");
    }
}
