//! Big file analysis - chunking and recursive condensation

use anyhow::Result;

use super::analyzer::{AnalysisResult, LlmUsage};
use crate::db::FileAnalysis;
use crate::llm::LlmClient;

/// Maximum tokens per chunk
const MAX_TOKENS_PER_CHUNK: usize = 6_000;

/// Condensation context limit
const CONDENSE_CONTEXT_LIMIT: usize = 12_000;

/// Small file threshold for dedup vs condensation
const SMALL_FILE_DEDUP_THRESHOLD: usize = 3;

/// Chunk result from analyzing a chunk
#[derive(Debug, Clone)]
struct ChunkResult {
    language: String,
    analysis: FileAnalysis,
}

/// Split content into chunks
fn split_into_chunks(content: &str, max_tokens: usize) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();
    let mut current_chunk = Vec::new();
    let mut current_tokens = 0;

    for line in lines {
        let line_tokens = estimate_tokens(line);
        if current_tokens + line_tokens > max_tokens && !current_chunk.is_empty() {
            chunks.push(current_chunk.join("\n"));
            current_chunk = Vec::new();
            current_tokens = 0;
        }
        current_chunk.push(line);
        current_tokens += line_tokens;
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk.join("\n"));
    }

    chunks
}

/// Analyze a chunk
async fn analyze_chunk(
    llm_client: &LlmClient,
    relative_path: &str,
    chunk_index: usize,
    total_chunks: usize,
    chunk_content: &str,
) -> Result<ChunkResult> {
    let prompt = format!(
        r#"You are analyzing chunk {} of {} from a single source file for a code knowledge graph.
Focus on what exists in THIS CHUNK only. Do not infer content from other chunks.
Return ONLY a JSON object, no prose, no markdown fences, with EXACTLY these keys:

- purpose          : string
- summary          : string
- businessContext  : string
- language         : string
- classes          : string[]
- functions        : string[]
- importsInternal  : string[]
- importsExternal  : string[]
- keywords         : string[]

File path: {}
Chunk content:
{}"#,
        chunk_index + 1,
        total_chunks,
        relative_path,
        chunk_content
    );

    let response = llm_client.complete(&prompt).await?;

    // Parse response (simplified)
    let raw: serde_json::Value = serde_json::from_str(&response.content)
        .unwrap_or_else(|_| serde_json::json!({}));

    Ok(ChunkResult {
        language: raw["language"].as_str().unwrap_or("unknown").to_string(),
        analysis: FileAnalysis {
            purpose: raw["purpose"].as_str().unwrap_or("").to_string(),
            summary: raw["summary"].as_str().unwrap_or("").to_string(),
            business_context: raw["businessContext"].as_str().unwrap_or("").to_string(),
            classes_defined: raw["classesDefined"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            functions_defined: raw["functionsDefined"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            keywords: raw["keywords"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            classes_used: raw["classesUsed"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            functions_used: raw["functionsUsed"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            modules_defined: raw["modulesDefined"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            modules_imported: raw["modulesImported"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            modules_external: raw["modulesExternal"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        },
    })
}

/// Merge chunk results using dedup (for small number of chunks)
fn dedup_merge(chunks: Vec<ChunkResult>) -> AnalysisResult {
    let language = chunks.iter()
        .find(|c| c.language != "unknown")
        .map(|c| c.language.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let mut purposes: Vec<String> = Vec::new();
    let mut summaries: Vec<String> = Vec::new();
    let mut contexts: Vec<String> = Vec::new();
    let mut classes_defined: Vec<String> = Vec::new();
    let mut functions_defined: Vec<String> = Vec::new();
    let mut keywords: Vec<String> = Vec::new();
    let mut modules_defined: Vec<String> = Vec::new();
    let mut classes_used: Vec<String> = Vec::new();
    let mut functions_used: Vec<String> = Vec::new();
    let mut modules_imported: Vec<String> = Vec::new();
    let mut modules_external: Vec<String> = Vec::new();

    for chunk in &chunks {
        if !chunk.analysis.purpose.is_empty() {
            purposes.push(chunk.analysis.purpose.clone());
        }
        if !chunk.analysis.summary.is_empty() {
            summaries.push(chunk.analysis.summary.clone());
        }
        if !chunk.analysis.business_context.is_empty() {
            contexts.push(chunk.analysis.business_context.clone());
        }
        // Collect defined entities
        classes_defined.extend(chunk.analysis.classes_defined.iter().cloned());
        functions_defined.extend(chunk.analysis.functions_defined.iter().cloned());
        keywords.extend(chunk.analysis.keywords.iter().cloned());
        modules_defined.extend(chunk.analysis.modules_defined.iter().cloned());
        // Collect used entities
        classes_used.extend(chunk.analysis.classes_used.iter().cloned());
        functions_used.extend(chunk.analysis.functions_used.iter().cloned());
        modules_imported.extend(chunk.analysis.modules_imported.iter().cloned());
        modules_external.extend(chunk.analysis.modules_external.iter().cloned());
    }

    // Deduplicate
    let dedup = |v: &mut Vec<String>| {
        v.sort();
        v.dedup();
        v.truncate(10);
    };

    dedup(&mut classes_defined);
    dedup(&mut functions_defined);
    dedup(&mut keywords);

    AnalysisResult {
        language,
        analysis: FileAnalysis {
            purpose: purposes.join(" | "),
            summary: summaries.join(" | "),
            business_context: contexts.join(" "),
            classes_defined,
            functions_defined,
            keywords,
            classes_used,
            functions_used,
            modules_defined,
            modules_imported,
            modules_external,
        },
        usage: LlmUsage {
            model: String::new(),
            input_tokens: 0,
            output_tokens: 0,
        },
    }
}

/// Estimate tokens (rough)
fn estimate_tokens(content: &str) -> usize {
    content.len() / 4
}

/// Analyze a big file
pub async fn analyze_big_file(
    llm_client: &LlmClient,
    relative_path: &str,
    content: &str,
) -> Result<AnalysisResult> {
    let chunks = split_into_chunks(content, MAX_TOKENS_PER_CHUNK);
    let total = chunks.len();

    if total <= SMALL_FILE_DEDUP_THRESHOLD {
        // Use dedup merge for small number of chunks
        let mut results = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            results.push(analyze_chunk(llm_client, relative_path, i, total, chunk).await?);
        }
        return Ok(dedup_merge(results));
    }

    // TODO: Implement recursive LLM condensation for larger files
    // For now, use dedup
    let mut results = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        results.push(analyze_chunk(llm_client, relative_path, i, total, chunk).await?);
    }
    Ok(dedup_merge(results))
}
