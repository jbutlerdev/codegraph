//! LLM prompt templates

/// File analysis prompt
pub static FILE_ANALYSIS_PROMPT: &str = r#"You are analyzing a single source file for a code knowledge graph.
Return ONLY a JSON object, no prose, no markdown fences, with EXACTLY these keys:

- purpose          : string  — Authoritative explanation of why this file exists and how it fits in the system. No speculation, no roadmap, no invented intent. Return empty string only if purpose cannot be inferred. Max ~300 tokens.
- summary          : string  — Natural language summary of the file's purpose, key patterns, architecture role, and important concepts for search and developer comprehension. Plain English paragraph. NO JSON, NO key-value pairs. Cover: what the file does, why it exists, key design patterns or algorithms, and how it fits in the system. Do NOT duplicate class/function names verbatim. Max 600 tokens.
- businessContext  : string  — Short paragraph (2-3 lines) describing the business/product domain this file serves, why it matters, and what breaks if it fails. Focus on business language, not technical implementation. Max ~100 tokens. Empty string if no business context can be inferred.
- language         : string  — Lowercase canonical name of any programming, markup, config, or data language identifiable from the contents (e.g. typescript, python, go, dockerfile, markdown, terraform, graphql). Return "unknown" if you cannot identify the language with confidence.
- classes          : string[] — Every structural/type definition in the file (classes, interfaces, enums, structs, unions, traits, etc.). Format: "ExactName (~L3-29): What it represents or controls". 8-15 words per entry. Exact names from source code, preserve original casing.
- functions        : string[] — Every function/method/procedure/callable definition in the file. Format: "exact_name (~L3-29): Primary responsibility". 8-15 words per entry. Exact names from source code, preserve original casing.
- importsInternal  : string[] — Relative imports only (./ or ../). Exact paths as written.
- importsExternal  : string[] — External packages or standard libraries only. Package names only (no paths).
- keywords         : string[] — Up to 10 technical domain keywords or short phrases for AI-powered search. Focus on: technologies, frameworks, domain concepts, algorithms, patterns, protocols. Use natural casing. No generic terms like "code", "file", "function".

IMPORTANT: For Python files, extract imported classes correctly:
- `from module import ClassName` → include "ClassName" in importsExternal
- `from .relative import Something` → include "Something" in importsInternal
- `import module` → include "module" in importsExternal

For JavaScript/TypeScript:
- `import { ClassName } from 'module'` → include "ClassName" in importsExternal if from external package
- `import { ClassName } from './file'` → include "ClassName" in importsInternal if from relative path

File path: {path}
File content:
{content}"#;

/// Chunk analysis prompt  
pub static CHUNK_ANALYSIS_PROMPT: &str = r#"You are analyzing chunk {chunk_num} of {total_chunks} from a single source file for a code knowledge graph.
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

File path: {path}
Chunk content:
{content}"#;

/// Condensation merge prompt
pub static CONDENSE_PROMPT: &str = r#"You are condensing {count} partial analyses of a single file into ONE coherent file-level analysis.
Return ONLY a JSON object, no prose, no markdown fences, with EXACTLY the same keys as each input item.

## Merge rules

- purpose          : merge into ONE cohesive 2-3 sentence description.
- summary          : ≤600 tokens, plain-English; cover what the file does, why it exists, and how it fits in the system.
- businessContext  : merge into ONE short paragraph (2-3 lines).
- language         : single canonical name; if items disagree, pick the value that appears most often.
- classes          : deduplicate. Keep ONLY exported / public / entry-point items. Aggressively filter to stay under ~3000 tokens total.
- functions        : deduplicate. Keep ONLY exported / public / entry-point items. Aggressively filter to stay under ~3000 tokens total.
- importsInternal  : deduplicate within the list.
- importsExternal  : deduplicate within the list. Drop stdlib and trivial utilities.
- keywords         : deduplicate, keep the top 10 most representative.

INPUT ({count} partial analyses):

{input}"#;

/// Build a file analysis prompt
pub fn build_analysis_prompt(path: &str, content: &str) -> String {
    FILE_ANALYSIS_PROMPT
        .replace("{path}", path)
        .replace("{content}", content)
}

/// Build a chunk analysis prompt
pub fn build_chunk_prompt(path: &str, chunk_num: usize, total_chunks: usize, content: &str) -> String {
    CHUNK_ANALYSIS_PROMPT
        .replace("{path}", path)
        .replace("{chunk_num}", &chunk_num.to_string())
        .replace("{total_chunks}", &total_chunks.to_string())
        .replace("{content}", content)
}

/// Build a condensation prompt
pub fn build_condense_prompt(items: &[(&str, &str)], total: usize) -> String {
    let input = items.iter()
        .enumerate()
        .map(|(i, (analysis, lang))| format!("--- Item {} (language: {}) ---\n{}", i + 1, lang, analysis))
        .collect::<Vec<_>>()
        .join("\n\n");

    CONDENSE_PROMPT
        .replace("{count}", &total.to_string())
        .replace("{input}", &input)
}
