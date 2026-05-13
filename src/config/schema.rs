//! Configuration schema and validation

use serde::{Deserialize, Serialize};

/// LLM API type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum LlmApiType {
    OpenAi,        // OpenAI-compatible /chat/completions
    Anthropic,     // Anthropic /v1/messages
    OpenAiResponses, // OpenAI Responses API
}

impl Default for LlmApiType {
    fn default() -> Self {
        LlmApiType::OpenAi
    }
}

/// Configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// LLM endpoint URL
    #[serde(default = "default_endpoint")]
    pub llm_endpoint: String,

    /// LLM API key
    #[serde(default)]
    pub llm_api_key: String,

    /// LLM model name
    #[serde(default = "default_model")]
    pub llm_model: String,

    /// LLM API type (openai, anthropic, openai-responses)
    #[serde(default)]
    pub llm_api_type: LlmApiType,

    /// Concurrency for file analysis
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,

    /// Maximum tokens per file before chunking (0 = disabled)
    #[serde(default = "default_max_file_tokens")]
    pub max_file_tokens: usize,

    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Log retention in days
    #[serde(default = "default_log_retention")]
    pub log_retention_days: u32,

    /// LLM caching enabled
    #[serde(default = "default_llm_cache")]
    pub llm_cache_enabled: bool,
}

fn default_endpoint() -> String {
    "http://localhost:8080/v1".to_string()
}

fn default_model() -> String {
    "gpt-4".to_string()
}

fn default_api_type() -> LlmApiType {
    LlmApiType::default()
}

fn default_concurrency() -> usize {
    4
}

fn default_max_file_tokens() -> usize {
    12_000 // Default to 12k tokens, 0 = disabled
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_retention() -> u32 {
    14
}

fn default_llm_cache() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm_endpoint: default_endpoint(),
            llm_api_key: String::new(),
            llm_model: default_model(),
            llm_api_type: default_api_type(),
            concurrency: default_concurrency(),
            max_file_tokens: default_max_file_tokens(),
            log_level: default_log_level(),
            log_retention_days: default_log_retention(),
            llm_cache_enabled: default_llm_cache(),
        }
    }
}

impl Config {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.llm_api_key.is_empty() {
            return Err("llm_api_key is required".to_string());
        }

        if self.llm_endpoint.is_empty() {
            return Err("llm_endpoint is required".to_string());
        }

        Ok(())
    }
}
