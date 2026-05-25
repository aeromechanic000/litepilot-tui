use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// A single diagnostic from the language server.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LspDiagnostic {
    pub file: String,
    pub line: u32,
    pub severity: String,
    pub message: String,
}

/// Lightweight LSP client that queries diagnostics for a single file.
/// Spawns the language server, sends initialize + didOpen + diagnostics,
/// then shuts down.
pub struct LspClient {
    server_cmd: String,
    server_args: Vec<String>,
}

impl LspClient {
    /// Create a client for rust-analyzer.
    pub fn rust_analyzer() -> Self {
        Self {
            server_cmd: "rust-analyzer".into(),
            server_args: vec![],
        }
    }

    /// Create a client for pyright (Python).
    pub fn pyright() -> Self {
        Self {
            server_cmd: "pyright".into(),
            server_args: vec![],
        }
    }

    /// Create a client for typescript-language-server.
    pub fn typescript() -> Self {
        Self {
            server_cmd: "typescript-language-server".into(),
            server_args: vec!["--stdio".into()],
        }
    }

    /// Detect the appropriate LSP client based on file extension.
    pub fn for_file(path: &Path) -> Option<Self> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => Some(Self::rust_analyzer()),
            Some("py") => Some(Self::pyright()),
            Some("ts" | "tsx" | "js" | "jsx") => Some(Self::typescript()),
            _ => None,
        }
    }

    /// Check if the language server binary is available.
    pub fn is_available(&self) -> bool {
        which_binary(&self.server_cmd).is_some()
    }

    /// Query diagnostics for a file. Spawns the language server,
    /// opens the file, and collects published diagnostics.
    pub fn diagnostics(&self, file_path: &Path, workspace: &Path) -> Result<Vec<LspDiagnostic>> {
        if !self.is_available() {
            anyhow::bail!("{} not found in PATH", self.server_cmd);
        }

        let mut child = Command::new(&self.server_cmd)
            .args(&self.server_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .current_dir(workspace)
            .spawn()
            .with_context(|| format!("Starting {}", self.server_cmd))?;

        // Send LSP initialize request
        let init_id = 1;
        let root_uri = path_to_uri(workspace);
        let init_params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {}
        });
        send_request(&mut child, init_id, "initialize", &init_params)?;

        // Read initialize response
        read_response(&mut child, init_id)?;

        // Send initialized notification
        send_notification(&mut child, "initialized", &serde_json::json!({}))?;

        // Send didOpen notification
        let file_uri = path_to_uri(file_path);
        let language_id = match file_path.extension().and_then(|e| e.to_str()) {
            Some("rs") => "rust",
            Some("py") => "python",
            Some("ts") => "typescript",
            Some("tsx") => "typescriptreact",
            Some("js") => "javascript",
            Some("jsx") => "javascriptreact",
            _ => "plaintext",
        };
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Reading {}", file_path.display()))?;
        let text_doc = serde_json::json!({
            "textDocument": {
                "uri": file_uri,
                "languageId": language_id,
                "version": 1,
                "text": content
            }
        });
        send_notification(&mut child, "textDocument/didOpen", &text_doc)?;

        // Read diagnostics from the server's output
        // Language servers publish diagnostics as notifications
        let diagnostics = read_diagnostics(&mut child, 5000)?;

        // Shutdown
        let shutdown_id = 2;
        send_request(&mut child, shutdown_id, "shutdown", &serde_json::json!({}))?;
        let _ = read_response(&mut child, shutdown_id);
        send_notification(&mut child, "exit", &serde_json::json!({}))?;

        let _ = child.wait();
        Ok(diagnostics)
    }
}

/// Find a binary in PATH.
fn which_binary(cmd: &str) -> Option<PathBuf> {
    let path_var = std::env::var("PATH").ok()?;
    for dir in path_var.split(':') {
        let candidate = PathBuf::from(dir).join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Convert a path to a file:// URI.
fn path_to_uri(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    format!("file://{}", canonical.display())
}

/// Send a JSON-RPC request to the language server.
fn send_request(
    child: &mut Child,
    id: u64,
    method: &str,
    params: &serde_json::Value,
) -> Result<()> {
    let message = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    });
    send_message(child, &message)
}

/// Send a JSON-RPC notification to the language server.
fn send_notification(child: &mut Child, method: &str, params: &serde_json::Value) -> Result<()> {
    let message = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    });
    send_message(child, &message)
}

/// Send a JSON-RPC message with Content-Length header.
fn send_message(child: &mut Child, message: &serde_json::Value) -> Result<()> {
    let body = serde_json::to_string(message)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(header.as_bytes())?;
        stdin.write_all(body.as_bytes())?;
        stdin.flush()?;
    }
    Ok(())
}

/// Read a JSON-RPC response from the language server.
fn read_response(child: &mut Child, _expected_id: u64) -> Result<serde_json::Value> {
    let stdout = child.stdout.as_mut().context("No stdout from LSP server")?;
    let mut reader = BufReader::new(stdout);
    read_one_message(&mut reader)
}

/// Read diagnostic notifications from the language server with a timeout.
fn read_diagnostics(child: &mut Child, timeout_ms: u64) -> Result<Vec<LspDiagnostic>> {
    let stdout = child.stdout.as_mut().context("No stdout from LSP server")?;
    let mut reader = BufReader::new(stdout);
    let mut all_diagnostics = Vec::new();

    // Read messages until timeout
    let start = std::time::Instant::now();
    let deadline = std::time::Duration::from_millis(timeout_ms);

    loop {
        if start.elapsed() > deadline {
            break;
        }

        match read_one_message(&mut reader) {
            Ok(msg) => {
                if let Some(params) = msg.get("params") {
                    if let Some(diags) = params.get("diagnostics") {
                        if let Some(diags_arr) = diags.as_array() {
                            for d in diags_arr {
                                let start_line =
                                    d.get("range")
                                        .and_then(|r| r.get("start"))
                                        .and_then(|s| s.get("line"))
                                        .and_then(|l| l.as_u64())
                                        .unwrap_or(0) as u32
                                        + 1;
                                let severity = d
                                    .get("severity")
                                    .and_then(|s| {
                                        s.as_u64().map(|v| match v {
                                            1 => "Error",
                                            2 => "Warning",
                                            3 => "Information",
                                            4 => "Hint",
                                            _ => "Unknown",
                                        })
                                    })
                                    .unwrap_or("Error")
                                    .to_string();
                                let message = d
                                    .get("message")
                                    .and_then(|m| m.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                let uri = params.get("uri").and_then(|u| u.as_str()).unwrap_or("");
                                let file = uri.trim_start_matches("file://").to_string();

                                all_diagnostics.push(LspDiagnostic {
                                    file,
                                    line: start_line,
                                    severity,
                                    message,
                                });
                            }
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }

    Ok(all_diagnostics)
}

/// Read a single JSON-RPC message (Content-Length header + body).
fn read_one_message(
    reader: &mut BufReader<&mut std::process::ChildStdout>,
) -> Result<serde_json::Value> {
    let mut header_line = String::new();
    let mut content_len: usize = 0;

    // Read headers
    loop {
        header_line.clear();
        let bytes = reader.read_line(&mut header_line)?;
        if bytes == 0 {
            anyhow::bail!("LSP server closed connection");
        }
        let line = header_line.trim();
        if line.is_empty() {
            break;
        }
        if let Some(len_str) = line.strip_prefix("Content-Length:") {
            content_len = len_str.trim().parse::<usize>()?;
        }
    }

    if content_len == 0 {
        anyhow::bail!("No Content-Length in LSP message");
    }

    let mut body = vec![0u8; content_len];
    reader.read_exact(&mut body)?;
    let value: serde_json::Value = serde_json::from_slice(&body)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsp_client_for_file() {
        assert!(LspClient::for_file(Path::new("main.rs")).is_some());
        assert!(LspClient::for_file(Path::new("app.py")).is_some());
        assert!(LspClient::for_file(Path::new("index.ts")).is_some());
        assert!(LspClient::for_file(Path::new("comp.tsx")).is_some());
        assert!(LspClient::for_file(Path::new("readme.md")).is_none());
        assert!(LspClient::for_file(Path::new("Makefile")).is_none());
    }

    #[test]
    fn path_to_uri_format() {
        let uri = path_to_uri(Path::new("/tmp/test.rs"));
        assert!(uri.starts_with("file://"));
        assert!(uri.ends_with("test.rs"));
    }

    #[test]
    fn rust_analyzer_constructor() {
        let client = LspClient::rust_analyzer();
        assert_eq!(client.server_cmd, "rust-analyzer");
    }

    #[test]
    fn pyright_constructor() {
        let client = LspClient::pyright();
        assert_eq!(client.server_cmd, "pyright");
    }

    #[test]
    fn typescript_constructor() {
        let client = LspClient::typescript();
        assert_eq!(client.server_cmd, "typescript-language-server");
        assert_eq!(client.server_args, vec!["--stdio"]);
    }
}
