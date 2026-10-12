//! Shared naming convention for captured battle-log files: `ptcgl-<nanos>.txt`.
//! The nanosecond epoch timestamp is embedded in the filename so capture time
//! survives file moves and lets the UI sort logs newest-first without reading
//! file metadata.

/// Build the capture filename for a nanosecond epoch timestamp.
pub fn log_file_name(nanos: u64) -> String {
    format!("ptcgl-{nanos}.txt")
}

/// Extract the nanosecond epoch timestamp from a capture filename.
/// Returns `None` for any name that is not a `ptcgl-<nanos>.txt` capture file.
pub fn parse_log_file_name(file_name: &str) -> Option<u64> {
    let stem = file_name.strip_prefix("ptcgl-")?.strip_suffix(".txt")?;
    stem.parse::<u64>().ok()
}
