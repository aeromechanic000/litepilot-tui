use crate::tools::{Tool, ToolDef, ToolResult};
use anyhow::Result;

pub struct ExecShell;

impl ExecShell {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for ExecShell {
    fn execute(&self, params: serde_json::Value, call_id: String) -> Result<ToolResult> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter"))?;

        // Validate the first command word against sandbox rules
        let first_word = command.split_whitespace().next().unwrap_or("");
        let sandbox = crate::sandbox::Sandbox::new(std::path::PathBuf::from("."));
        if let Err(e) = sandbox.validate_command(first_word, &[]) {
            return Ok(ToolResult::err(
                "exec_shell",
                call_id,
                format!("Command rejected: {}", e),
            ));
        }

        // Use /bin/sh -c so shell features (redirects, pipes, heredocs) work
        let output = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(command)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let mut result = stdout.to_string();
                if !stderr.is_empty() {
                    result.push_str(&format!("\n[stderr]\n{}", stderr));
                }
                if !output.status.success() {
                    return Ok(ToolResult::err("exec_shell", call_id, result));
                }
                Ok(ToolResult::ok("exec_shell", call_id, result))
            }
            Err(e) => Ok(ToolResult::err(
                "exec_shell",
                call_id,
                format!("Execution failed: {}", e),
            )),
        }
    }

    fn definition(&self) -> ToolDef {
        ToolDef {
            name: "exec_shell".into(),
            description:
                "Execute a shell command. Commands are validated against a safety allowlist.".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command to run (e.g. 'cargo test')" }
                },
                "required": ["command"]
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_fields() {
        let def = ExecShell::new().definition();
        assert_eq!(def.name, "exec_shell");
        assert!(!def.description.is_empty());
    }

    #[test]
    fn execute_blocked_command() {
        let tool = ExecShell::new();
        let result = tool
            .execute(serde_json::json!({"command": "sudo rm -rf /"}), "c1".into())
            .unwrap();
        assert!(!result.success);
    }
}
