//! Configuration file I/O

use crate::config::Config;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get the config directory path (~/.codegraph)
pub fn config_dir() -> Result<PathBuf> {
    let dir = dirs::home_dir()
        .context("Cannot determine home directory")?
        .join(".codegraph");

    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .context("Cannot create config directory")?;
    }

    Ok(dir)
}

/// Get the config file path (~/.codegraph/config.toml)
pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// Get the database path (~/.codegraph/codegraph.db)
pub fn db_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("codegraph.db"))
}

/// Get the repos directory (~/.codegraph/repos)
pub fn repos_dir() -> Result<PathBuf> {
    let dir = config_dir()?.join("repos");

    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .context("Cannot create repos directory")?;
    }

    Ok(dir)
}

/// Get the queue data directory (~/.codegraph/queue)
pub fn queue_dir() -> Result<PathBuf> {
    let dir = config_dir()?.join("queue");

    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .context("Cannot create queue directory")?;
    }

    Ok(dir)
}

/// Get the logs directory (~/.codegraph/logs)
pub fn logs_dir() -> Result<PathBuf> {
    let dir = config_dir()?.join("logs");

    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .context("Cannot create logs directory")?;
    }

    Ok(dir)
}

/// Load configuration from file
pub fn load_config() -> Result<Config> {
    let path = config_path()?;

    if !path.exists() {
        return Ok(Config::default());
    }

    let contents = std::fs::read_to_string(&path)
        .context("Cannot read config file")?;

    toml::from_str(&contents)
        .context("Cannot parse config file")
}

/// Save configuration to file
pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;

    let contents = toml::to_string_pretty(config)
        .context("Cannot serialize config")?;

    std::fs::write(&path, contents)
        .context("Cannot write config file")?;

    // Set file permissions to 0600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)
            .context("Cannot get file permissions")?
            .permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)
            .context("Cannot set file permissions")?;
    }

    Ok(())
}

/// Get a specific config value
pub fn get_config_value(key: &str) -> Result<String> {
    let config = load_config()?;

    match key {
        "llm_endpoint" => Ok(config.llm_endpoint.clone()),
        "llm_api_key" => Ok(config.llm_api_key.clone()),
        "llm_model" => Ok(config.llm_model.clone()),
        "llm_api_type" => Ok(format!("{:?}", config.llm_api_type).to_lowercase()),
        "concurrency" => Ok(config.concurrency.to_string()),
        "max_file_tokens" => Ok(config.max_file_tokens.to_string()),
        "log_level" => Ok(config.log_level.clone()),
        "log_retention_days" => Ok(config.log_retention_days.to_string()),
        "llm_cache_enabled" => Ok(config.llm_cache_enabled.to_string()),
        _ => Err(anyhow::anyhow!("Unknown config key: {}", key)),
    }
}

/// Set a specific config value
pub fn set_config_value(key: &str, value: &str) -> Result<()> {
    let mut config = load_config()?;

    match key {
        "llm_endpoint" => {
            config.llm_endpoint = value.to_string();
        },
        "llm_api_key" => {
            config.llm_api_key = value.to_string();
        },
        "llm_model" => {
            config.llm_model = value.to_string();
        },
        "llm_api_type" => {
            config.llm_api_type = match value.to_lowercase().as_str() {
                "openai" => crate::config::LlmApiType::OpenAi,
                "anthropic" => crate::config::LlmApiType::Anthropic,
                "openai-responses" | "openai_responses" => crate::config::LlmApiType::OpenAiResponses,
                _ => return Err(anyhow::anyhow!("Invalid API type: {}. Use: openai, anthropic, openai-responses", value)),
            };
        },
        "concurrency" => {
            config.concurrency = value.parse()
                .map_err(|_| anyhow::anyhow!("Invalid concurrency value"))?;
        },
        "max_file_tokens" => {
            config.max_file_tokens = value.parse()
                .map_err(|_| anyhow::anyhow!("Invalid max_file_tokens value"))?;
        },
        "log_level" => {
            config.log_level = value.to_string();
        },
        "log_retention_days" => {
            config.log_retention_days = value.parse()
                .map_err(|_| anyhow::anyhow!("Invalid log retention value"))?;
        },
        "llm_cache_enabled" => {
            config.llm_cache_enabled = value.parse()
                .map_err(|_| anyhow::anyhow!("Invalid boolean value"))?;
        },
        _ => return Err(anyhow::anyhow!("Unknown config key: {}", key)),
    }

    save_config(&config)?;
    Ok(())
}
