use crate::ollama::OllamaClient;
use anyhow::{Context, Result};
use futures::StreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    think: bool,
    options: ChatOptions,
}

#[derive(Debug, Clone, Serialize)]
struct ChatOptions {
    /// Context window size in tokens
    num_ctx: u64,
    /// -1 = unlimited output tokens
    num_predict: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    pub content: String,
    pub done: bool,
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    #[allow(dead_code)]
    pub model: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    #[allow(dead_code)]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

impl OllamaClient {
    pub async fn chat(&self, model: &str, messages: Vec<ChatMessage>, think: bool) -> Result<ChatResponse> {
        let url = format!("{}/api/chat", self.endpoint);
        let body = ChatRequest {
            model: model.to_string(),
            messages,
            stream: false,
            think,
            options: ChatOptions { num_ctx: self.num_ctx, num_predict: -1 },
        };

        let resp = self
            .http
            .post(&url)
            .timeout(std::time::Duration::from_secs(300))
            .json(&body)
            .send()
            .await
            .with_context(|| format!("Sending chat request to Ollama at {}", url))?;

        if resp.status() == StatusCode::NOT_FOUND {
            anyhow::bail!("Model '{}' not found in Ollama", model);
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama chat error {}: {}", status, text);
        }

        let raw: serde_json::Value = resp.json().await?;
        let content = raw
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        let resp_model = raw
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or(model)
            .to_string();

        Ok(ChatResponse {
            content,
            model: resp_model,
        })
    }

    pub fn chat_stream(
        http: reqwest::Client,
        endpoint: String,
        model: String,
        messages: Vec<ChatMessage>,
        think: bool,
        num_ctx: u64,
        cancel: watch::Receiver<bool>,
    ) -> impl futures::Stream<Item = Result<ChatChunk>> {
        let url = format!("{}/api/chat", endpoint);
        let body = ChatRequest {
            model: model.clone(),
            messages,
            stream: true,
            think,
            options: ChatOptions { num_ctx, num_predict: -1 },
        };

        async_stream::stream! {
            let resp = match http.post(&url).json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    yield Err(anyhow::anyhow!("Stream connect failed: {}", e));
                    return;
                }
            };

            if !resp.status().is_success() {
                let status = resp.status();
                yield Err(anyhow::anyhow!("Ollama stream error: {}", status));
                return;
            }

            let mut stream = resp.bytes_stream();
            let mut buffer = String::new();
            let read_timeout = std::time::Duration::from_secs(300);

            loop {
                let cancelled = *cancel.borrow();
                if cancelled {
                    yield Ok(ChatChunk {
                        content: String::new(),
                        done: true,
                        model: model.clone(),
                    });
                    return;
                }

                let chunk_result = match tokio::time::timeout(read_timeout, stream.next()).await {
                    Ok(Some(result)) => result,
                    Ok(None) => {
                        // Stream ended without done:true — yield a final done chunk
                        // so the consumer receives the content and can finish
                        yield Ok(ChatChunk {
                            content: String::new(),
                            done: true,
                            model: model.clone(),
                        });
                        return;
                    }
                    Err(_) => {
                        yield Err(anyhow::anyhow!("Stream timed out (no data for 300s)"));
                        return;
                    }
                };

                let bytes = match chunk_result {
                    Ok(b) => b,
                    Err(e) => {
                        yield Err(anyhow::anyhow!("Stream read error: {}", e));
                        return;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<serde_json::Value>(&line) {
                        Ok(val) => {
                            let content = val.get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string();
                            let done = val.get("done")
                                .and_then(|d| d.as_bool())
                                .unwrap_or(false);
                            let resp_model = val.get("model")
                                .and_then(|m| m.as_str())
                                .unwrap_or(&model)
                                .to_string();

                            yield Ok(ChatChunk {
                                content,
                                done,
                                model: resp_model,
                            });

                            if done {
                                return;
                            }
                        }
                        Err(e) => {
                            yield Err(anyhow::anyhow!("JSON parse error: {} in line: {}", e, line));
                            return;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_message_constructors() {
        let sys = ChatMessage::system("You are a helpful assistant.");
        assert_eq!(sys.role, "system");
        let user = ChatMessage::user("Write a hello world.");
        assert_eq!(user.role, "user");
        let asst = ChatMessage::assistant("Here is the code:");
        assert_eq!(asst.role, "assistant");
    }

    #[test]
    fn chat_request_serialization() {
        let req = ChatRequest {
            model: "qwen3:4b".to_string(),
            messages: vec![ChatMessage::user("test")],
            stream: true,
            think: true,
            options: ChatOptions { num_ctx: 262144, num_predict: -1 },
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"stream\":true"));
        assert!(json.contains("\"model\":\"qwen3:4b\""));
        assert!(json.contains("\"num_predict\":-1"));
    }

    #[tokio::test]
    async fn chat_model_not_found() {
        let config = crate::config::Config {
            ollama_endpoint: "http://localhost:19999".into(),
            ..crate::config::Config::default()
        };
        let client = OllamaClient::new(&config).unwrap();
        let result = client
            .chat("nonexistent", vec![ChatMessage::user("hi")], true)
            .await;
        assert!(result.is_err());
    }
}
