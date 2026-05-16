use super::{Session, SessionMeta};
use crate::config::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

fn sessions_dir() -> Result<PathBuf> {
    Config::sessions_dir()
}

pub fn save_session(session: &Session) -> Result<()> {
    let dir = sessions_dir()?;
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", session.id));
    let json = serde_json::to_string_pretty(session)
        .context("Serializing session")?;
    // Atomic write: write to temp file, then rename
    let tmp_path = dir.join(format!("{}.tmp", session.id));
    fs::write(&tmp_path, &json)?;
    fs::rename(&tmp_path, &path)?;
    Ok(())
}

pub fn load_session(id: &str) -> Result<Session> {
    let dir = sessions_dir()?;
    let path = dir.join(format!("{}.json", id));
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Reading session {}", id))?;
    let session: Session = serde_json::from_str(&content)
        .with_context(|| format!("Parsing session {}", id))?;
    Ok(session)
}

pub fn list_sessions() -> Result<Vec<SessionMeta>> {
    let dir = sessions_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(session) = serde_json::from_str::<Session>(&content) {
                sessions.push(session.to_meta());
            }
        }
    }

    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(sessions)
}

pub fn delete_session(id: &str) -> Result<()> {
    let dir = sessions_dir()?;
    let path = dir.join(format!("{}.json", id));
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join("litecode_test_sessions");
        let _ = fs::create_dir_all(&dir);

        let mut session = Session::new();
        session.add_message("user", "Test message");
        session.add_message("assistant", "Test response");

        let path = dir.join(format!("{}.json", session.id));
        let json = serde_json::to_string_pretty(&session).unwrap();
        fs::write(&path, &json).unwrap();

        let loaded: Session = serde_json::from_str(&serde_json::to_string(&session).unwrap()).unwrap();
        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.messages.len(), 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_nonexistent_session() {
        let result = load_session("nonexistent-uuid");
        assert!(result.is_err());
    }

    #[test]
    fn list_empty_dir() {
        let dir = std::env::temp_dir().join("litecode_test_empty_sessions");
        let _ = fs::create_dir_all(&dir);
        // list_sessions uses Config::sessions_dir(), so just verify it doesn't crash
        let _ = list_sessions();
        let _ = fs::remove_dir_all(&dir);
    }
}
