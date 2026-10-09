use crate::types::{Config, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;

#[derive(Clone)]
pub struct Cancellation {
    sender: Arc<watch::Sender<bool>>,
}
impl Default for Cancellation {
    fn default() -> Self {
        let (sender, _) = watch::channel(false);
        Self {
            sender: Arc::new(sender),
        }
    }
}
impl Cancellation {
    pub fn cancel(&self) {
        self.sender.send_replace(true);
    }
    pub fn is_cancelled(&self) -> bool {
        *self.sender.borrow()
    }
    pub async fn cancelled(&self) {
        let mut receiver = self.sender.subscribe();
        if *receiver.borrow_and_update() {
            return;
        }
        loop {
            if receiver.changed().await.is_err() || *receiver.borrow_and_update() {
                return;
            }
        }
    }
}
#[derive(Clone, Serialize)]
pub struct GenerationRequest {
    pub system: String,
    pub prompt: String,
    pub config: Config,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerationResponse {
    pub content: String,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ProviderCapabilities {
    pub seed: bool,
    pub usage: bool,
}
#[async_trait]
pub trait ModelProvider: Send + Sync {
    fn identity(&self) -> String;
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }
    async fn generate(&self, request: &GenerationRequest) -> Result<GenerationResponse>;
}
pub struct OpenAiCompatible {
    client: reqwest::Client,
    endpoint: reqwest::Url,
    key: Option<String>,
    capabilities: ProviderCapabilities,
}
impl OpenAiCompatible {
    pub fn new(
        endpoint: &str,
        key: Option<String>,
        capabilities: ProviderCapabilities,
    ) -> Result<Self> {
        let endpoint = reqwest::Url::parse(endpoint).map_err(|_| "invalid endpoint")?;
        if !matches!(endpoint.scheme(), "http" | "https")
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
        {
            return Err("endpoint must be HTTP(S), without credentials, query or fragment".into());
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "HTTP client initialization failed")?;
        Ok(Self {
            client,
            endpoint,
            key,
            capabilities,
        })
    }
}
#[async_trait]
impl ModelProvider for OpenAiCompatible {
    fn identity(&self) -> String {
        self.endpoint.to_string()
    }
    fn capabilities(&self) -> ProviderCapabilities {
        self.capabilities
    }
    async fn generate(&self, req: &GenerationRequest) -> Result<GenerationResponse> {
        let mut body = serde_json::json!({"model":req.config.model,"temperature":req.config.temperature,
            "max_tokens":req.config.max_tokens,"messages":[{"role":"system","content":req.system},
                {"role":"user","content":req.prompt}]});
        if self.capabilities.seed {
            if let Some(seed) = req.config.seed {
                body["seed"] = seed.into();
            }
        }
        let mut request = self
            .client
            .post(self.endpoint.clone())
            .json(&body)
            .timeout(std::time::Duration::from_millis(req.config.timeout_ms));
        if let Some(key) = &self.key {
            request = request.bearer_auth(key);
        }
        let mut response = request
            .send()
            .await
            .map_err(|_| "provider transport failure")?;
        if !response.status().is_success() {
            return Err(format!(
                "provider HTTP status {}",
                response.status().as_u16()
            ));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "provider response failure")?
        {
            if bytes.len().saturating_add(chunk.len()) > req.config.max_response_bytes {
                return Err("provider response exceeds byte limit".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| "invalid provider JSON")?;
        if value["choices"][0]["finish_reason"] == "length" {
            return Err("provider output truncated by token limit".into());
        }
        let content = value["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("missing provider content")?
            .to_owned();
        Ok(GenerationResponse {
            content,
            prompt_tokens: value["usage"]["prompt_tokens"].as_u64(),
            completion_tokens: value["usage"]["completion_tokens"].as_u64(),
        })
    }
}
/// Deterministic fixture provider. Scripted responses are test evidence, not live inference.
pub struct ScriptedProvider {
    responses: Mutex<std::collections::VecDeque<String>>,
}
impl ScriptedProvider {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
        }
    }
}
#[async_trait]
impl ModelProvider for ScriptedProvider {
    fn identity(&self) -> String {
        "scripted-fixture".into()
    }
    async fn generate(&self, _: &GenerationRequest) -> Result<GenerationResponse> {
        let content = self
            .responses
            .lock()
            .map_err(|_| "fixture lock failure")?
            .pop_front()
            .ok_or("fixture response script exhausted")?;
        Ok(GenerationResponse {
            content,
            prompt_tokens: None,
            completion_tokens: None,
        })
    }
}
