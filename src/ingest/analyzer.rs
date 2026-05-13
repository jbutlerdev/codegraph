//! File analyzer - LLM-based analysis of source files

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::info;

use crate::db::FileAnalysis;
use crate::llm::LlmClient;
use crate::config::load_config;

/// File analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub language: String,
    pub analysis: FileAnalysis,
    pub usage: LlmUsage,
}

/// LLM usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmUsage {
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Analysis prompt template
const ANALYSIS_PROMPT_TEMPLATE: &str = r#"You are analyzing a single source file for a code knowledge graph.
Return ONLY a JSON object, no prose, no markdown fences, with EXACTLY these keys:

- purpose          : string  — Authoritative explanation of why this file exists and how it fits in the system. No speculation, no roadmap, no invented intent. Return empty string only if purpose cannot be inferred. Max ~300 tokens.
- summary          : string  — Natural language summary of the file's purpose, key patterns, architecture role, and important concepts for search and developer comprehension. Plain English paragraph. NO JSON, NO key-value pairs. Cover: what the file does, why it exists, key design patterns or algorithms, and how it fits in the system. Do NOT duplicate class/function names verbatim. Max 600 tokens.
- businessContext  : string  — Short paragraph (2-3 lines) describing the business/product domain this file serves, why it matters, and what breaks if it fails. Focus on business language, not technical implementation. Max ~100 tokens. Empty string if no business context can be inferred.
- language         : string  — Lowercase canonical name of any programming, markup, config, or data language identifiable from the contents (e.g. typescript, python, go, dockerfile, markdown, terraform, graphql). Return "unknown" if you cannot identify the language with confidence.

# Entities DEFINED in this file (the file creates/exports these):
- classesDefined   : string[] — Structural/type definitions declared in THIS file (classes, interfaces, enums, structs, unions, traits, etc.). Format: "ExactName (~L3-29): What it represents". Exact names from source code.
- functionsDefined : string[] — Functions/methods/procedures defined in THIS file. Format: "exact_name (~L3-29): Primary responsibility". Exact names from source code.
- modulesDefined   : string[] — Module names this file defines (e.g. Go package name, Rust mod, Python __name__, etc.).
- keywords         : string[] — Up to 10 technical domain keywords for search. Focus on: technologies, frameworks, domain concepts, algorithms, patterns, protocols.

# Entities USED/IMPORTED by this file (the file depends on these):
- classesUsed      : string[] — Structural/type definitions from OTHER files that this file instantiates, extends, or uses. Format: "ClassName: Where it's imported from or context".
- functionsUsed    : string[] — Functions from OTHER files that this file calls. Format: "function_name: Where it's imported from or context".
- modulesImported  : string[] — Relative imports (./ or ../). Exact paths as written.
- modulesExternal  : string[] — External packages/libraries imported. Package names only.

File path: {path}
File content:
{content}"#;

/// Default big file threshold (12,000 tokens)
const DEFAULT_BIG_FILE_THRESHOLD: usize = 12_000;

/// Analyze a single file
pub async fn analyze_file(
    llm_client: &LlmClient,
    relative_path: &str,
    content: &str,
) -> Result<AnalysisResult> {
    let tokens = estimate_tokens(content);
    
    // Get threshold from config, default to 12k if not set
    let threshold = load_config()
        .map(|c| c.max_file_tokens)
        .unwrap_or(DEFAULT_BIG_FILE_THRESHOLD);

    // If threshold is 0, chunking is disabled
    if threshold > 0 && tokens > threshold {
        info!("File {} is large ({} tokens, threshold: {}), using chunked analysis", 
              relative_path, tokens, threshold);
        return analyze_big_file(llm_client, relative_path, content).await;
    }

    info!("Analyzing file {} ({} tokens)", relative_path, tokens);

    let prompt = ANALYSIS_PROMPT_TEMPLATE
        .replace("{path}", relative_path)
        .replace("{content}", content);

    let response = llm_client.complete(&prompt).await?;

    let parsed = parse_analysis_response(&response.content)?;
    let usage = LlmUsage {
        model: response.model.clone(),
        input_tokens: response.input_tokens,
        output_tokens: response.output_tokens,
    };

    Ok(AnalysisResult {
        language: parsed.language,
        analysis: parsed.analysis,
        usage,
    })
}

/// Analyze a large file by chunking
async fn analyze_big_file(
    _llm_client: &LlmClient,
    _relative_path: &str,
    _content: &str,
) -> Result<AnalysisResult> {
    // TODO: Implement chunking and recursive condensation
    // For now, just return empty analysis
    Ok(AnalysisResult {
        language: "unknown".to_string(),
        analysis: FileAnalysis::default(),
        usage: LlmUsage {
            model: String::new(),
            input_tokens: 0,
            output_tokens: 0,
        },
    })
}

/// Parse the LLM response into an analysis
fn parse_analysis_response(response: &str) -> Result<AnalysisResult> {
    // Try to extract JSON from response
    let json_str = extract_json(response)?;

    #[derive(Deserialize)]
    struct RawAnalysis {
        purpose: Option<String>,
        summary: Option<String>,
        business_context: Option<String>,
        language: Option<String>,
        // Defined entities
        classes_defined: Option<Vec<String>>,
        functions_defined: Option<Vec<String>>,
        modules_defined: Option<Vec<String>>,
        keywords: Option<Vec<String>>,
        // Used entities
        classes_used: Option<Vec<String>>,
        functions_used: Option<Vec<String>>,
        modules_imported: Option<Vec<String>>,
        modules_external: Option<Vec<String>>,
    }

    let raw: RawAnalysis = serde_json::from_str(json_str)
        .context("Failed to parse LLM response as JSON")?;

    Ok(AnalysisResult {
        language: raw.language.unwrap_or_else(|| "unknown".to_string()),
        analysis: FileAnalysis {
            purpose: raw.purpose.unwrap_or_default(),
            summary: raw.summary.unwrap_or_default(),
            business_context: raw.business_context.unwrap_or_default(),
            classes_defined: raw.classes_defined.unwrap_or_default(),
            functions_defined: raw.functions_defined.unwrap_or_default(),
            modules_defined: raw.modules_defined.unwrap_or_default(),
            keywords: raw.keywords.unwrap_or_default(),
            classes_used: raw.classes_used.unwrap_or_default(),
            functions_used: raw.functions_used.unwrap_or_default(),
            modules_imported: raw.modules_imported.unwrap_or_default(),
            modules_external: raw.modules_external.unwrap_or_default(),
        },
        usage: LlmUsage {
            model: String::new(),
            input_tokens: 0,
            output_tokens: 0,
        },
    })
}

/// Extract JSON from a response that might have markdown fences
fn extract_json(response: &str) -> Result<&str> {
    let trimmed = response.trim();

    // Check for markdown code fences
    if trimmed.starts_with("```json") {
        if let Some(end) = trimmed.find("```") {
            return Ok(&trimmed[7..end].trim());
        }
    }

    if trimmed.starts_with("```") {
        if let Some(end) = trimmed.find("```") {
            return Ok(&trimmed[3..end].trim());
        }
    }

    // Try parsing the whole thing
    Ok(trimmed)
}

/// Estimate token count (rough approximation: 4 chars per token)
fn estimate_tokens(content: &str) -> usize {
    content.len() / 4
}

/// Compute SHA256 of content
pub fn compute_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Detect language from file extension
pub fn detect_language(path: &str) -> Option<String> {
    let ext = path.rsplit('.').next()?;
    let lang = match ext.to_lowercase().as_str() {
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "rs" => "rust",
        "py" | "pyw" => "python",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "cs" => "csharp",
        "cpp" | "cc" | "cxx" | "c" | "h" | "hpp" => "cpp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "scala" => "scala",
        "md" | "markdown" => "markdown",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" | "sass" | "less" => "css",
        "sql" => "sql",
        "sh" | "bash" | "zsh" => "shell",
        "dockerfile" => "dockerfile",
        "tf" => "terraform",
        "proto" => "protobuf",
        "graphql" | "gql" => "graphql",
        "vue" => "vue",
        "svelte" => "svelte",
        _ => return None,
    };
    Some(lang.to_string())
}
