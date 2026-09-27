use std::fs;
use std::path::PathBuf;

use crate::logname::log_file_name;
use crate::ports::LogStore;

/// Saves battle-log text to a timestamped `.txt` file in a directory.
pub struct FsLogStore {
    dir: PathBuf,
}

impl FsLogStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }
}

impl LogStore for FsLogStore {
    fn save(&self, text: &str) -> Result<String, String> {
        fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let path = self.dir.join(log_file_name(nanos));
        fs::write(&path, text).map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().to_string())
    }
}