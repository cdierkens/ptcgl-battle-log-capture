use std::path::PathBuf;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_appender::non_blocking::WorkerGuard;

use crate::settings;

/// Initializes structured logging to stderr and a daily-rolling file at
/// ~/.ptcgl-battle-logs/logs/capture.log. Returns a guard that must be kept
/// alive for the app's lifetime so buffered log lines are flushed on exit.
pub fn init() -> WorkerGuard {
    let log_dir: PathBuf = settings::base_dir().join("logs");
    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = tracing_appender::rolling::daily(&log_dir, "capture.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let stderr_layer = tracing_subscriber::fmt::layer().with_target(false).compact();
    let file_layer = tracing_subscriber::fmt::layer().with_ansi(false).with_writer(non_blocking);

    tracing_subscriber::registry()
        .with(filter)
        .with(stderr_layer)
        .with(file_layer)
        .init();

    guard
}