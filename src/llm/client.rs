//! LLM API client (OpenAI-compatible and Anthropic)

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info};

use crate::config::{Config, LlmApiType};

/// LLM response
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Chat completion request (OpenAI format)
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

/// Chat message
#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Chat completion response (OpenAI format)
#[derive(Debug, Deserialize)]
struct ChatResponse {
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

/// Anthropic messages request
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// Anthropic messages response
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// LLM client for OpenAI-compatible and Anthropic APIs
pub struct LlmClient {
    client: Client,
    endpoint: String,
    api_key: String,
    model: String,
    api_type: LlmApiType,
}

impl LlmClient {
    /// Create a new LLM client from config
    pub fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(360))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            endpoint: config.llm_endpoint.clone(),
            api_key: config.llm_api_key.clone(),
            model: config.llm_model.clone(),
            api_type: config.llm_api_type.clone(),
        })
    }

    /// Send a completion request
    pub async fn complete(&self, prompt: &str) -> Result<LlmResponse> {
        self.complete_with_max_tokens(prompt, Some(4096)).await
    }

    /// Complete with optional max tokens limit
    pub async fn complete_with_max_tokens(&self, prompt: &str, max_tokens: Option<u32>) -> Result<LlmResponse> {
        info!("Sending request to LLM endpoint: {} with model: {} (api_type: {:?})", 
              self.endpoint, self.model, self.api_type);

        match self.api_type {
            LlmApiType::Anthropic => {
                self.anthropic_complete(prompt, max_tokens.unwrap_or(4096)).await
            },
            LlmApiType::OpenAi | LlmApiType::OpenAiResponses => {
                // OpenAI Responses API uses same /chat/completions endpoint format
                self.openai_complete(prompt, max_tokens).await
            },
        }
    }

    /// OpenAI-compatible chat completions
    async fn openai_complete(&self, prompt: &str, max_tokens: Option<u32>) -> Result<LlmResponse> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens,
        };

        let response = self.client
            .post(format!("{}/chat/completions", self.endpoint.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("LLM API error {}: {}", status, text));
        }

        let chat: ChatResponse = response.json().await
            .context("Failed to parse response")?;

        let content = chat.choices.first()
            .and_then(|c| Some(c.message.content.clone()))
            .unwrap_or_default();

        if content.is_empty() {
            return Err(anyhow::anyhow!("LLM returned empty completion"));
        }

        debug!("LLM response: {} tokens in, {} tokens out", 
            chat.usage.prompt_tokens, chat.usage.completion_tokens);

        Ok(LlmResponse {
            content,
            model: chat.model,
            input_tokens: chat.usage.prompt_tokens,
            output_tokens: chat.usage.completion_tokens,
        })
    }

    /// Anthropic messages API
    async fn anthropic_complete(&self, prompt: &str, max_tokens: u32) -> Result<LlmResponse> {
        let request = AnthropicRequest {
            model: self.model.clone(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens,
        };

        let response = self.client
            .post(format!("{}/v1/messages", self.endpoint.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await
            .context("Failed to send request")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("LLM API error {}: {}", status, text));
        }

        let anthropic_resp: AnthropicResponse = response.json().await
            .context("Failed to parse Anthropic response")?;

        // Extract text from content blocks
        let content = anthropic_resp.content.iter()
            .filter(|c| c.content_type == "text")
            .map(|c| c.text.clone().unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");

        if content.is_empty() {
            return Err(anyhow::anyhow!("LLM returned empty completion"));
        }

        debug!("LLM response: {} tokens in, {} tokens out", 
            anthropic_resp.usage.input_tokens, anthropic_resp.usage.output_tokens);

        Ok(LlmResponse {
            content,
            model: self.model.clone(),
            input_tokens: anthropic_resp.usage.input_tokens,
            output_tokens: anthropic_resp.usage.output_tokens,
        })
    }
}
