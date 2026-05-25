use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

/// Maximum age of log files in days before they are cleaned up on startup.
const MAX_LOG_AGE_DAYS: i64 = 30;

/// Initialize file-based logging for this session.
///
/// Returns the `WorkerGuard` (must be held for the lifetime of the app)
/// and the path to the log file.
pub fn init(config_dir: &Path) -> (tracing_appender::non_blocking::WorkerGuard, PathBuf) {
    let log_dir = config_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);

    cleanup_old_logs(&log_dir);

    let filename = format!(
        "session_{}.log",
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
    );
    let log_path = log_dir.join(&filename);

    let file = std::fs::File::create(&log_path).expect("Failed to create log file");
    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .with_target(false)
        .with_ansi(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    (guard, log_path)
}

/// Remove log files older than MAX_LOG_AGE_DAYS.
fn cleanup_old_logs(log_dir: &Path) {
    let cutoff = chrono::Local::now() - chrono::Duration::days(MAX_LOG_AGE_DAYS);
    let Ok(entries) = std::fs::read_dir(log_dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("log") {
            continue;
        }
        if let Ok(metadata) = entry.metadata() {
            if let Ok(modified) = metadata.modified() {
                let modified_time: chrono::DateTime<chrono::Local> = modified.into();
                if modified_time < cutoff {
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }
}
