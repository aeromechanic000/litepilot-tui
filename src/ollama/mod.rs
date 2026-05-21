pub mod chat;
pub mod model;

use crate::config::Config;
use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

pub struct OllamaClient {
    endpoint: String,
    #[allow(dead_code)]
    timeout: Duration,
    http: Client,
    num_ctx: u64,
}

impl OllamaClient {
    pub fn new(config: &Config) -> Result<Self> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(config.connect_timeout))
            .timeout(Duration::from_secs(600))
            .build()
            .context("Creating HTTP client")?;
        Ok(Self {
            endpoint: config.ollama_endpoint.trim_end_matches('/').to_string(),
            timeout: Duration::from_secs(config.connect_timeout),
            http,
            num_ctx: config.context_window_limit,
        })
    }

    #[allow(dead_code)]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Build an HTTP client suitable for streaming (no overall deadline, only connect timeout).
    pub fn streaming_http_client(connect_timeout: Duration) -> Result<Client> {
        Client::builder()
            .connect_timeout(connect_timeout)
            // No .timeout() — streams can run indefinitely; per-chunk read timeout
            // is handled by the OS TCP stack (typically ~60s between chunks).
            .build()
            .context("Creating streaming HTTP client")
    }

    pub async fn ping(&self) -> Result<()> {
        let url = format!("{}/api/tags", self.endpoint);
        let resp = self
            .http
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .with_context(|| format!("Connecting to Ollama at {}", self.endpoint))?;
        if !resp.status().is_success() {
            anyhow::bail!("Ollama returned status {}", resp.status());
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn list_models(&self) -> Result<Vec<model::ModelInfo>> {
        let url = format!("{}/api/tags", self.endpoint);
        let resp = self
            .http
            .get(&url)
            .timeout(self.timeout)
            .send()
            .await
            .context("Fetching model list from Ollama")?;

        if !resp.status().is_success() {
            anyhow::bail!("Ollama returned status {}", resp.status());
        }

        let body: serde_json::Value = resp.json().await?;
        let models = body
            .get("models")
            .and_then(|m| m.as_array())
            .cloned()
            .unwrap_or_default();

        let mut result = Vec::new();
        for m in models {
            let name = m
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let size = m.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
            let quantization_level = m
                .get("details")
                .and_then(|d| d.get("quantization_level"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let family = m
                .get("details")
                .and_then(|d| d.get("family"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let parameter_count = model::estimate_parameters(&name);
            let size_class = model::classify_model(parameter_count);
            let context_window = model::estimate_context_window(&name);

            result.push(model::ModelInfo {
                name,
                size,
                parameter_count,
                quantization_level,
                family,
                size_class,
                context_window,
            });
        }

        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            ollama_endpoint: "http://localhost:11434".into(),
            connect_timeout: 5,
            ..Config::default()
        }
    }

    #[tokio::test]
    async fn ping_connection_refused() {
        let config = Config {
            ollama_endpoint: "http://localhost:19999".into(),
            ..test_config()
        };
        let client = OllamaClient::new(&config).unwrap();
        let result = client.ping().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_models_connection_refused() {
        let config = Config {
            ollama_endpoint: "http://localhost:19999".into(),
            ..test_config()
        };
        let client = OllamaClient::new(&config).unwrap();
        let result = client.list_models().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn client_constructs_with_valid_config() {
        let config = test_config();
        let client = OllamaClient::new(&config);
        assert!(client.is_ok());
        assert_eq!(client.unwrap().endpoint(), "http://localhost:11434");
    }
}
