use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

/// Structured events emitted during agent operation.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum HookEvent {
    TurnStarted {
        model: String,
        mode: String,
        input_len: usize,
        timestamp: String,
    },
    ToolCalled {
        tool: String,
        params_summary: String,
        step: usize,
        timestamp: String,
    },
    ToolResult {
        tool: String,
        success: bool,
        duration_ms: u64,
        timestamp: String,
    },
    TurnComplete {
        model: String,
        output_len: usize,
        attempts: usize,
        duration_ms: u64,
        file_changes: usize,
        timestamp: String,
    },
    Error {
        message: String,
        source: String,
        timestamp: String,
    },
}

impl HookEvent {
    pub fn turn_started(model: &str, mode: &str, input_len: usize) -> Self {
        Self::TurnStarted {
            model: model.to_string(),
            mode: mode.to_string(),
            input_len,
            timestamp: now_iso(),
        }
    }

    #[allow(dead_code)]
    pub fn tool_called(tool: &str, params_summary: &str, step: usize) -> Self {
        Self::ToolCalled {
            tool: tool.to_string(),
            params_summary: params_summary.to_string(),
            step,
            timestamp: now_iso(),
        }
    }

    #[allow(dead_code)]
    pub fn tool_result(tool: &str, success: bool, duration_ms: u64) -> Self {
        Self::ToolResult {
            tool: tool.to_string(),
            success,
            duration_ms,
            timestamp: now_iso(),
        }
    }

    pub fn turn_complete(
        model: &str,
        output_len: usize,
        attempts: usize,
        duration_ms: u64,
        file_changes: usize,
    ) -> Self {
        Self::TurnComplete {
            model: model.to_string(),
            output_len,
            attempts,
            duration_ms,
            file_changes,
            timestamp: now_iso(),
        }
    }

    pub fn error(message: &str, source: &str) -> Self {
        Self::Error {
            message: message.to_string(),
            source: source.to_string(),
            timestamp: now_iso(),
        }
    }
}

/// JSONL event sink that appends structured events to a file.
pub struct JsonlSink {
    writer: Mutex<std::fs::File>,
}

impl JsonlSink {
    /// Open or create the JSONL sink at the given path.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            writer: Mutex::new(file),
        })
    }

    /// Write an event as a single JSONL line.
    pub fn emit(&self, event: &HookEvent) {
        if let Ok(line) = serde_json::to_string(event) {
            if let Ok(mut writer) = self.writer.lock() {
                let _ = writeln!(writer, "{}", line);
            }
        }
    }

    /// Convenience: emit a TurnStarted event.
    pub fn turn_started(&self, model: &str, mode: &str, input_len: usize) {
        self.emit(&HookEvent::turn_started(model, mode, input_len));
    }

    /// Convenience: emit a TurnComplete event.
    pub fn turn_complete(
        &self,
        model: &str,
        output_len: usize,
        attempts: usize,
        duration_ms: u64,
        file_changes: usize,
    ) {
        self.emit(&HookEvent::turn_complete(
            model,
            output_len,
            attempts,
            duration_ms,
            file_changes,
        ));
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn event_serialization() {
        let event = HookEvent::turn_started("qwen3:8b", "edit", 42);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"TurnStarted\""));
        assert!(json.contains("\"model\":\"qwen3:8b\""));
        assert!(json.contains("\"input_len\":42"));
    }

    #[test]
    fn tool_called_event() {
        let event = HookEvent::tool_called("write_file", "src/main.rs", 1);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"ToolCalled\""));
        assert!(json.contains("\"tool\":\"write_file\""));
        assert!(json.contains("\"step\":1"));
    }

    #[test]
    fn tool_result_event() {
        let event = HookEvent::tool_result("write_file", true, 150);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"ToolResult\""));
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"duration_ms\":150"));
    }

    #[test]
    fn turn_complete_event() {
        let event = HookEvent::turn_complete("qwen3:8b", 1024, 2, 3500, 3);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"TurnComplete\""));
        assert!(json.contains("\"output_len\":1024"));
        assert!(json.contains("\"file_changes\":3"));
    }

    #[test]
    fn error_event() {
        let event = HookEvent::error("connection refused", "ollama");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"Error\""));
        assert!(json.contains("\"source\":\"ollama\""));
    }

    #[test]
    fn jsonl_sink_writes() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");
        let sink = JsonlSink::open(&path).unwrap();

        sink.turn_started("qwen3:8b", "edit", 10);
        sink.turn_complete("qwen3:8b", 500, 1, 1000, 0);

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.trim().lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"type\":\"TurnStarted\""));
        assert!(lines[1].contains("\"type\":\"TurnComplete\""));
    }

    #[test]
    fn jsonl_sink_appends() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("events.jsonl");

        {
            let sink = JsonlSink::open(&path).unwrap();
            sink.turn_started("model-a", "auto", 5);
        }
        {
            let sink = JsonlSink::open(&path).unwrap();
            sink.turn_started("model-b", "plan", 10);
        }

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.trim().lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("model-a"));
        assert!(lines[1].contains("model-b"));
    }

    #[test]
    fn sink_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("deep/nested/events.jsonl");
        let sink = JsonlSink::open(&path).unwrap();
        sink.turn_started("test", "edit", 1);
        assert!(path.exists());
    }
}
