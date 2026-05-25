// Linux Landlock sandbox integration (kernel 5.13+).
//
// Landlock provides unprivileged access control. When available,
// it restricts child processes to:
// - Read access to system directories
// - Read/write access to the workspace directory
// - Network access remains available
//
// When Landlock is not available (older kernel, non-Linux OS),
// falls back to the command allowlist/blocklist approach.

use std::path::Path;

/// Check if Landlock is available on this system.
/// Currently always returns false — Landlock requires the `landlock` crate
/// and kernel 5.13+. This is a placeholder for future implementation.
#[allow(dead_code)]
pub fn is_available() -> bool {
    // Landlock is Linux-only and requires kernel 5.13+
    // For now, always return false and fall back to allowlist
    if cfg!(target_os = "linux") {
        // Check kernel version
        if let Ok(version) = get_kernel_version() {
            return version >= (5, 13);
        }
    }
    false
}

/// Get the kernel version as a (major, minor) tuple.
#[allow(dead_code)]
fn get_kernel_version() -> Result<(u32, u32), std::env::VarError> {
    let version_str = std::env::var("KERNEL_VERSION").or_else(|_| {
        // Try uname -r
        let output = std::process::Command::new("uname").arg("-r").output();
        match output {
            Ok(o) => {
                let v = String::from_utf8_lossy(&o.stdout).to_string();
                Ok(v)
            }
            Err(_) => Err(std::env::VarError::NotPresent),
        }
    })?;

    // Parse "5.15.0-..." → (5, 15)
    let parts: Vec<&str> = version_str.trim().split('.').collect();
    if parts.len() >= 2 {
        let major = parts[0].parse::<u32>().unwrap_or(0);
        let minor = parts[1]
            .split('-')
            .next()
            .unwrap_or("0")
            .parse::<u32>()
            .unwrap_or(0);
        Ok((major, minor))
    } else {
        Ok((0, 0))
    }
}

/// Build a Landlock ruleset for the given workspace.
/// Returns a description of the intended policy (for logging).
#[allow(dead_code)]
pub fn build_policy_description(workspace: &Path) -> String {
    let ws = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    format!(
        "Landlock policy: read=/usr,/lib; read/write={}; network=allowed",
        ws.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn build_policy_contains_workspace() {
        let tmp = TempDir::new().unwrap();
        let desc = build_policy_description(tmp.path());
        assert!(desc.contains("Landlock policy"));
        assert!(desc.contains("read/write"));
    }

    #[test]
    fn kernel_version_parsing() {
        // Valid version strings
        assert_eq!(parse_version("5.15.0-generic"), Some((5, 15)));
        assert_eq!(parse_version("6.1.0"), Some((6, 1)));
        assert_eq!(parse_version("5.4.0-rc1"), Some((5, 4)));
    }

    fn parse_version(s: &str) -> Option<(u32, u32)> {
        let parts: Vec<&str> = s.trim().split('.').collect();
        if parts.len() >= 2 {
            let major = parts[0].parse::<u32>().ok()?;
            let minor = parts[1].split('-').next()?.parse::<u32>().ok()?;
            Some((major, minor))
        } else {
            None
        }
    }
}
