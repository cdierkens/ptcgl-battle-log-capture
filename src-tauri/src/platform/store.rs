use std::fs;
use std::path::PathBuf;

use crate::logname::log_file_name;
use crate::ports::LogStore;
use crate::settings::{self, SharedSettings};

/// Saves battle-log text to a timestamped `.txt` file in the configured output directory,
/// read per save so a newly picked folder applies immediately.
pub struct FsLogStore {
    settings: SharedSettings,
}

impl FsLogStore {
    pub fn new(settings: SharedSettings) -> Self {
        Self { settings }
    }
}

impl LogStore for FsLogStore {
    fn save(&self, text: &str) -> Result<String, String> {
        let dir = PathBuf::from(settings::current(&self.settings).output_dir);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let path = dir.join(log_file_name(nanos));
        fs::write(&path, text).map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().to_string())
    }
}
