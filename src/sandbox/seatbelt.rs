// macOS Seatbelt sandbox integration using `sandbox-exec`.
//
// Generates a Seatbelt profile that restricts child processes to:
// - Read access to system directories (/usr, /lib, /System)
// - Read/write access to the workspace directory
// - Network access (for Ollama API calls)
// - No access to other user directories

use std::path::Path;
use std::process::Command;

/// Build a Seatbelt sandbox-exec profile for the given workspace.
fn build_profile(workspace: &Path) -> String {
    let ws = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let ws_str = ws.display();

    format!(
        r#"(version 1)
(deny default)
(allow file-read* (subpath "/usr") (subpath "/lib") (subpath "/System") (subpath "/dev"))
(allow file-read* (subpath "/tmp") (subpath "/private/tmp"))
(allow file-read* (subpath "{}"))
(allow file-write* (subpath "{}"))
(allow network*)
(allow process-exec (subpath "/usr") (subpath "/bin") (subpath "/sbin"))
(allow process-exec (subpath "/opt") (subpath "/usr/local"))
(allow process-fork)
(allow signal)
(allow sysctl-read)
(allow file-read-metadata)
"#,
        ws_str, ws_str
    )
}

/// Execute a command within a macOS Seatbelt sandbox.
/// Falls back to unsandboxed execution if sandbox-exec is not available.
pub fn exec_sandboxed(
    cmd: &str,
    args: &[String],
    workspace: &Path,
) -> Result<std::process::Output, std::io::Error> {
    let profile = build_profile(workspace);

    // Try sandbox-exec first
    let result = Command::new("/usr/bin/sandbox-exec")
        .arg("-p")
        .arg(&profile)
        .arg(cmd)
        .args(args)
        .output();

    match result {
        Ok(output) => Ok(output),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // sandbox-exec not available — fall back to unsandboxed
            tracing::warn!("sandbox-exec not found, falling back to unsandboxed execution");
            Command::new(cmd).args(args).output()
        }
        Err(e) => Err(e),
    }
}

/// Check if macOS Seatbelt sandboxing is available.
pub fn is_available() -> bool {
    std::path::Path::new("/usr/bin/sandbox-exec").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn profile_contains_workspace() {
        let tmp = TempDir::new().unwrap();
        let profile = build_profile(tmp.path());
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(deny default)"));
        assert!(profile.contains("(allow network*)"));
        // Should contain the workspace path
        let ws_canonical = tmp.path().canonicalize().unwrap();
        assert!(profile.contains(&ws_canonical.display().to_string()));
    }

    #[test]
    fn profile_allows_read_write() {
        let tmp = TempDir::new().unwrap();
        let profile = build_profile(tmp.path());
        assert!(profile.contains("file-read*"));
        assert!(profile.contains("file-write*"));
    }

    #[test]
    fn is_available_matches_binary() {
        // Should be true on macOS with developer tools, false otherwise
        let expected = std::path::Path::new("/usr/bin/sandbox-exec").exists();
        assert_eq!(is_available(), expected);
    }
}
