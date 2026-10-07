use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_player_name")]
    pub player_name: String,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    /// Auto-open the drawer and raise the Continue shield on the results screen.
    #[serde(default = "default_true")]
    pub prompt_after_match: bool,
    /// Save a battle log copied to the clipboard while the results screen is up.
    #[serde(default)]
    pub clipboard_autosave: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            player_name: default_player_name(),
            output_dir: default_output_dir(),
            prompt_after_match: true,
            clipboard_autosave: false,
        }
    }
}

/// Settings shared by commands and the engine.
pub type SharedSettings = Arc<RwLock<Settings>>;

/// A snapshot. A poisoned lock still holds the last written value.
pub fn current(shared: &SharedSettings) -> Settings {
    match shared.read() {
        Ok(s) => s.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

pub fn replace(shared: &SharedSettings, next: Settings) {
    let mut guard = match shared.write() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = next;
}

fn default_player_name() -> String {
    "cdierkens".to_string()
}

fn default_output_dir() -> String {
    base_dir().to_string_lossy().to_string()
}

fn default_true() -> bool {
    true
}

pub fn settings_path() -> PathBuf {
    base_dir().join("settings.json")
}

/// No settings file yet means first-run setup hasn't finished.
pub fn exists() -> bool {
    settings_path().exists()
}

pub fn load() -> Settings {
    fs::read_to_string(settings_path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(settings: &Settings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

pub fn base_dir() -> PathBuf {
    home_dir().join(".ptcgl-battle-logs")
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_settings_files_load_with_the_new_defaults() {
        let s: Settings = serde_json::from_str(r#"{"player_name":"ash","output_dir":"/logs"}"#).unwrap();
        assert_eq!(s.player_name, "ash");
        assert_eq!(s.output_dir, "/logs");
        assert!(s.prompt_after_match);
        assert!(!s.clipboard_autosave);
    }

    #[test]
    fn toggles_round_trip() {
        let s = Settings { prompt_after_match: false, clipboard_autosave: true, ..Settings::default() };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn replace_then_current() {
        let shared: SharedSettings = Arc::new(RwLock::new(Settings::default()));
        replace(&shared, Settings { clipboard_autosave: true, ..Settings::default() });
        assert!(current(&shared).clipboard_autosave);
    }
}
