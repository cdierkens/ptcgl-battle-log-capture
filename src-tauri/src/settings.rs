use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::destinations::Destination;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub player_name: String,
    /// Empty in a file means "the default log folder"; `SettingsPaths::load` fills it in.
    #[serde(default)]
    pub output_dir: String,
    /// Auto-open the drawer and raise the Continue shield on the results screen.
    #[serde(default = "default_true")]
    pub prompt_after_match: bool,
    /// Save a battle log copied to the clipboard while the results screen is up.
    #[serde(default)]
    pub clipboard_autosave: bool,
    /// Changed only through the destination commands; `save_settings` keeps the current list.
    #[serde(default)]
    pub destinations: Vec<Destination>,
}

impl Settings {
    pub fn defaults(log_folder: &Path) -> Self {
        Self {
            player_name: String::new(),
            output_dir: log_folder.to_string_lossy().into_owned(),
            prompt_after_match: true,
            clipboard_autosave: false,
            destinations: Vec::new(),
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

fn default_true() -> bool {
    true
}

/// Where settings and the default log folder live (ADR 0002).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsPaths {
    /// `<app config>/settings.json`.
    pub file: PathBuf,
    /// `<home>/.ptcgl-battle-logs`. Also holds the app's own diagnostic logs.
    pub log_folder: PathBuf,
}

impl SettingsPaths {
    pub fn new(app_config_dir: &Path, home_dir: &Path) -> Self {
        Self { file: app_config_dir.join("settings.json"), log_folder: home_dir.join(".ptcgl-battle-logs") }
    }

    pub fn resolve(app: &tauri::AppHandle) -> tauri::Result<Self> {
        use tauri::Manager;
        let path = app.path();
        Ok(Self::new(&path.app_config_dir()?, &path.home_dir()?))
    }

    /// No settings file yet means first-run setup hasn't finished.
    pub fn exists(&self) -> bool {
        self.file.exists()
    }

    pub fn load(&self) -> Settings {
        let mut settings = fs::read_to_string(&self.file)
            .ok()
            .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
            .unwrap_or_else(|| Settings::defaults(&self.log_folder));
        if settings.output_dir.is_empty() {
            settings.output_dir = self.log_folder.to_string_lossy().into_owned();
        }
        settings
    }

    pub fn save(&self, settings: &Settings) -> Result<(), String> {
        if let Some(parent) = self.file.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        fs::write(&self.file, json).map_err(|e| e.to_string())
    }
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
        let s =
            Settings { prompt_after_match: false, clipboard_autosave: true, ..Settings::defaults(Path::new("/logs")) };
        let back: Settings = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn replace_then_current() {
        let defaults = Settings::defaults(Path::new("/logs"));
        let shared: SharedSettings = Arc::new(RwLock::new(defaults.clone()));
        replace(&shared, Settings { clipboard_autosave: true, ..defaults });
        assert!(current(&shared).clipboard_autosave);
    }

    fn temp_paths(name: &str) -> (PathBuf, SettingsPaths) {
        let root = std::env::temp_dir().join(format!("ptcgl-settings-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let paths = SettingsPaths::new(&root.join("config"), &root.join("home"));
        (root, paths)
    }

    #[test]
    fn settings_live_in_app_config_and_logs_in_home() {
        let paths = SettingsPaths::new(Path::new("/config/dev.dierkens.ptcgl-tracker"), Path::new("/Users/ash"));
        assert_eq!(paths.file, Path::new("/config/dev.dierkens.ptcgl-tracker/settings.json"));
        assert_eq!(paths.log_folder, Path::new("/Users/ash/.ptcgl-battle-logs"));
    }

    #[test]
    fn first_run_loads_defaults_with_no_player_name() {
        let (root, paths) = temp_paths("first-run");
        assert!(!paths.exists());
        let s = paths.load();
        assert_eq!(s, Settings::defaults(&paths.log_folder));
        assert!(s.player_name.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn save_then_load_round_trips_and_marks_setup_done() {
        let (root, paths) = temp_paths("round-trip");
        let s = Settings { player_name: "ash".into(), ..Settings::defaults(&paths.log_folder) };
        paths.save(&s).unwrap();
        assert!(paths.exists());
        assert_eq!(paths.load(), s);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_file_without_an_output_dir_uses_the_log_folder() {
        let (root, paths) = temp_paths("no-output-dir");
        fs::create_dir_all(paths.file.parent().unwrap()).unwrap();
        fs::write(&paths.file, r#"{"player_name":"ash"}"#).unwrap();
        assert_eq!(paths.load().output_dir, paths.log_folder.to_string_lossy());
        let _ = fs::remove_dir_all(root);
    }
}
