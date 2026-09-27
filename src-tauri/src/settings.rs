use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_player_name")]
    pub player_name: String,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            player_name: default_player_name(),
            output_dir: default_output_dir(),
        }
    }
}

fn default_player_name() -> String {
    "cdierkens".to_string()
}

fn default_output_dir() -> String {
    base_dir().to_string_lossy().to_string()
}

pub fn settings_path() -> PathBuf {
    base_dir().join("settings.json")
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