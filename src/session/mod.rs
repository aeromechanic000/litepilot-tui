pub mod persistence;

use crate::ollama::chat::ChatMessage;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Session {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<SessionMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SessionMeta {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
    pub preview: String,
}

#[allow(dead_code)]
impl Session {
    pub fn new() -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now.clone(),
            updated_at: now,
            messages: Vec::new(),
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        let now = chrono::Utc::now().to_rfc3339();
        self.messages.push(SessionMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: now,
        });
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn user_messages(&self) -> Vec<ChatMessage> {
        self.messages.iter().map(|m| ChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        }).collect()
    }

    pub fn preview(&self) -> String {
        self.messages
            .first()
            .map(|m| m.content.chars().take(80).collect())
            .unwrap_or_default()
    }

    pub fn to_meta(&self) -> SessionMeta {
        SessionMeta {
            id: self.id.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            message_count: self.messages.len(),
            preview: self.preview(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_session_has_uuid() {
        let session = Session::new();
        assert!(!session.id.is_empty());
        assert!(session.messages.is_empty());
    }

    #[test]
    fn add_messages() {
        let mut session = Session::new();
        session.add_message("user", "Hello");
        session.add_message("assistant", "Hi there!");
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[1].content, "Hi there!");
    }

    #[test]
    fn preview_from_first_message() {
        let mut session = Session::new();
        session.add_message("user", "Create a web server with authentication");
        assert!(session.preview().contains("Create a web server"));
    }

    #[test]
    fn to_meta_conversion() {
        let mut session = Session::new();
        session.add_message("user", "test");
        let meta = session.to_meta();
        assert_eq!(meta.message_count, 1);
        assert_eq!(meta.id, session.id);
    }

    #[test]
    fn user_messages_conversion() {
        let mut session = Session::new();
        session.add_message("system", "You are helpful");
        session.add_message("user", "Hello");
        let msgs = session.user_messages();
        assert_eq!(msgs.len(), 2);
    }
}
