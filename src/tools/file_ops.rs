use crate::sandbox::Sandbox;
use crate::tools::{Tool, ToolDef, ToolResult};
use anyhow::Result;
use std::sync::Arc;

pub struct ReadFile {
    sandbox: Arc<Sandbox>,
}

impl ReadFile {
    pub fn new(sandbox: Arc<Sandbox>) -> Self {
        Self { sandbox }
    }
}

impl Tool for ReadFile {
    fn execute(&self, params: serde_json::Value, call_id: String) -> Result<ToolResult> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;
        let full = self.sandbox.workspace().join(path);
        match self.sandbox.validate_path(&full) {
            Ok(validated) => match std::fs::read_to_string(&validated) {
                Ok(content) => Ok(ToolResult::ok("read_file", call_id, content)),
                Err(e) => Ok(ToolResult::err(
                    "read_file",
                    call_id,
                    format!("Read failed: {}", e),
                )),
            },
            Err(e) => Ok(ToolResult::err("read_file", call_id, format!("{}", e))),
        }
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "read_file".into(),
            description: "Read file contents. Use to examine source code, config files, or docs."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to the file (e.g. 'src/main.rs')" }
                },
                "required": ["path"]
            }),
        }
    }
}

pub struct WriteFile {
    sandbox: Arc<Sandbox>,
}

impl WriteFile {
    pub fn new(sandbox: Arc<Sandbox>) -> Self {
        Self { sandbox }
    }
}

impl Tool for WriteFile {
    fn execute(&self, params: serde_json::Value, call_id: String) -> Result<ToolResult> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;
        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' parameter"))?;
        let full = self.sandbox.workspace().join(path);

        // Create parent dirs first so validate_path can canonicalize
        if let Some(parent) = full.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match self.sandbox.validate_path(&full) {
            Ok(validated) => match std::fs::write(&validated, content) {
                Ok(_) => Ok(ToolResult::ok(
                    "write_file",
                    call_id,
                    format!("Written {} bytes to {}", content.len(), path),
                )),
                Err(e) => Ok(ToolResult::err(
                    "write_file",
                    call_id,
                    format!("Write failed: {}", e),
                )),
            },
            Err(e) => Ok(ToolResult::err("write_file", call_id, format!("{}", e))),
        }
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "write_file".into(),
            description:
                "Create or overwrite a file. Parent directories are created automatically.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path (e.g. 'src/utils.rs')" },
                    "content": { "type": "string", "description": "File content to write" }
                },
                "required": ["path", "content"]
            }),
        }
    }
}

pub struct EditFile {
    sandbox: Arc<Sandbox>,
}

impl EditFile {
    pub fn new(sandbox: Arc<Sandbox>) -> Self {
        Self { sandbox }
    }
}

impl Tool for EditFile {
    fn execute(&self, params: serde_json::Value, call_id: String) -> Result<ToolResult> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;
        let old_text = params
            .get("old_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'old_text' parameter"))?;
        let new_text = params
            .get("new_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_text' parameter"))?;
        let full = self.sandbox.workspace().join(path);
        match self.sandbox.validate_path(&full) {
            Ok(validated) => {
                let content = match std::fs::read_to_string(&validated) {
                    Ok(c) => c,
                    Err(e) => {
                        return Ok(ToolResult::err(
                            "edit_file",
                            call_id,
                            format!("Read failed: {}", e),
                        ))
                    }
                };
                if !content.contains(old_text) {
                    return Ok(ToolResult::err(
                        "edit_file",
                        call_id,
                        "old_text not found in file",
                    ));
                }
                let new_content = content.replacen(old_text, new_text, 1);
                match std::fs::write(&validated, new_content) {
                    Ok(_) => Ok(ToolResult::ok(
                        "edit_file",
                        call_id,
                        format!("Edited {}", path),
                    )),
                    Err(e) => Ok(ToolResult::err(
                        "edit_file",
                        call_id,
                        format!("Write failed: {}", e),
                    )),
                }
            }
            Err(e) => Ok(ToolResult::err("edit_file", call_id, format!("{}", e))),
        }
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "edit_file".into(),
            description: "Replace exact text in a file. First occurrence of old_text is replaced."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to the file" },
                    "old_text": { "type": "string", "description": "Exact text to find (must match exactly)" },
                    "new_text": { "type": "string", "description": "Replacement text" }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        }
    }
}

pub struct ListDir {
    sandbox: Arc<Sandbox>,
}

impl ListDir {
    pub fn new(sandbox: Arc<Sandbox>) -> Self {
        Self { sandbox }
    }
}

impl Tool for ListDir {
    fn execute(&self, params: serde_json::Value, call_id: String) -> Result<ToolResult> {
        let path = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let full = self.sandbox.workspace().join(path);
        match self.sandbox.validate_path(&full) {
            Ok(validated) => {
                let entries = std::fs::read_dir(&validated)
                    .map(|rd| {
                        rd.filter_map(|e| e.ok())
                            .map(|e| {
                                let name = e.file_name().to_string_lossy().to_string();
                                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                                format!("{}{}", name, if is_dir { "/" } else { "" })
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_else(|e| format!("Error listing: {}", e));
                Ok(ToolResult::ok("list_dir", call_id, entries))
            }
            Err(e) => Ok(ToolResult::err("list_dir", call_id, format!("{}", e))),
        }
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "list_dir".into(),
            description: "List files and directories. Defaults to current directory.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path to list (default: '.')" }
                }
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, Arc<Sandbox>) {
        let dir = TempDir::new().unwrap();
        let sandbox = Arc::new(Sandbox::new(dir.path().to_path_buf()));
        (dir, sandbox)
    }

    #[test]
    fn read_file_tool() {
        let (dir, sandbox) = setup();
        std::fs::write(dir.path().join("test.txt"), "Hello, world!").unwrap();
        let tool = ReadFile::new(sandbox);
        let result = tool
            .execute(serde_json::json!({"path": "test.txt"}), "c1".into())
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output, "Hello, world!");
    }

    #[test]
    fn read_file_not_found() {
        let (_dir, sandbox) = setup();
        let tool = ReadFile::new(sandbox);
        let result = tool
            .execute(serde_json::json!({"path": "nonexistent.txt"}), "c1".into())
            .unwrap();
        assert!(!result.success);
    }

    #[test]
    fn write_file_tool() {
        let (dir, sandbox) = setup();
        let tool = WriteFile::new(sandbox);
        let result = tool
            .execute(
                serde_json::json!({"path": "new.txt", "content": "content"}),
                "c1".into(),
            )
            .unwrap();
        assert!(result.success);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("new.txt")).unwrap(),
            "content"
        );
    }

    #[test]
    fn write_file_creates_dirs() {
        let (dir, sandbox) = setup();
        let tool = WriteFile::new(sandbox);
        let result = tool
            .execute(
                serde_json::json!({"path": "sub/dir/file.txt", "content": "deep"}),
                "c1".into(),
            )
            .unwrap();
        assert!(result.success);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("sub/dir/file.txt")).unwrap(),
            "deep"
        );
    }

    #[test]
    fn edit_file_tool() {
        let (dir, sandbox) = setup();
        std::fs::write(dir.path().join("code.rs"), "fn old() {}").unwrap();
        let tool = EditFile::new(sandbox);
        let result = tool
            .execute(
                serde_json::json!({"path": "code.rs", "old_text": "fn old() {}", "new_text": "fn new() {}"}),
                "c1".into(),
            )
            .unwrap();
        assert!(result.success);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("code.rs")).unwrap(),
            "fn new() {}"
        );
    }

    #[test]
    fn edit_file_old_text_not_found() {
        let (dir, sandbox) = setup();
        std::fs::write(dir.path().join("code.rs"), "unchanged").unwrap();
        let tool = EditFile::new(sandbox);
        let result = tool
            .execute(
                serde_json::json!({"path": "code.rs", "old_text": "missing", "new_text": "replacement"}),
                "c1".into(),
            )
            .unwrap();
        assert!(!result.success);
    }

    #[test]
    fn list_dir_tool() {
        let (dir, sandbox) = setup();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("main.rs"), "").unwrap();
        let tool = ListDir::new(sandbox);
        let result = tool.execute(serde_json::json!({}), "c1".into()).unwrap();
        assert!(result.success);
        assert!(result.output.contains("src/"));
        assert!(result.output.contains("main.rs"));
    }

    #[test]
    fn definitions_have_required_fields() {
        let (dir, sandbox) = setup();
        for tool in [
            ReadFile::new(sandbox.clone()).definition(),
            WriteFile::new(sandbox.clone()).definition(),
            EditFile::new(sandbox.clone()).definition(),
            ListDir::new(sandbox).definition(),
        ] {
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());
            assert!(tool.parameters.is_object());
        }
    }
}
