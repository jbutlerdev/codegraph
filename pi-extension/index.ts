/**
 * CodeGraph Extension for pi
 * 
 * Provides tools for querying a local code knowledge graph with:
 * - Full-text search across LLM-generated summaries
 * - Entity lookup (keywords, classes, functions)
 * - Text grep across files
 * - File content viewing
 * - Repository management
 */

import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { Type } from "typebox";
import { exec } from "node:child_process";
import { promisify } from "node:util";

const execAsync = promisify(exec);

// Path to codegraph binary
const CODEGRAPH_BIN = "/data/jbutler/git/jbutlerdev/codegraph/target/release/codegraph";

/**
 * Execute codegraph CLI and return stdout
 */
async function runCodegraph(args: string[]): Promise<string> {
  try {
    const { stdout, stderr } = await execAsync(`${CODEGRAPH_BIN} ${args.join(" ")} 2>&1`, {
      timeout: 30000,
    });
    return stdout || stderr;
  } catch (error: unknown) {
    if (error && typeof error === "object" && "stdout" in error) {
      return String((error as { stdout?: unknown }).stdout || "");
    }
    throw error;
  }
}

/**
 * Parse codegraph output to extract structured data
 */
function parseSearchOutput(output: string): Array<{
  path: string;
  purpose: string;
  summary: string;
}> {
  const results: Array<{ path: string; purpose: string; summary: string }> = [];
  const lines = output.split("\n");
  
  let current: { path: string; purpose: string; summary: string } | null = null;
  let section: "none" | "purpose" | "summary" = "none";
  
  for (const line of lines) {
    // Match file path lines (no leading whitespace, ends with known extension or no extension)
    const pathMatch = line.match(/^([^\s].*\.(?:rs|toml|md|json|ts|js|py|go|yaml|yml|sh|txt))$/);
    if (pathMatch && !line.includes("results")) {
      if (current) {
        results.push(current);
      }
      current = { path: pathMatch[1], purpose: "", summary: "" };
      section = "none";
      continue;
    }
    
    if (!current) continue;
    
    if (line.includes("Purpose:")) {
      section = "purpose";
      current.purpose = line.replace(/^\s*Purpose:\s*/, "").trim();
    } else if (line.includes("Summary:")) {
      section = "summary";
      current.summary = line.replace(/^\s*Summary:\s*/, "").trim();
    } else if (section === "purpose" && line.match(/^\s{4,}/)) {
      current.purpose += " " + line.trim();
    } else if (section === "summary" && line.match(/^\s{4,}/)) {
      current.summary += " " + line.trim();
    } else if (line.match(/^\s*---/) || line.match(/^SEARCH RESULTS|^LOOKUP RESULTS|^GREP|^FILE/)) {
      section = "none";
    }
  }
  
  if (current) {
    results.push(current);
  }
  
  return results;
}

export default function (pi: ExtensionAPI) {
  // Notify on load
  pi.on("session_start", async (_event, ctx) => {
    ctx.ui.notify("CodeGraph extension loaded", "info");
  });

  // ============================================
  // Tool: codegraph_list_repos
  // ============================================
  pi.registerTool({
    name: "codegraph_list_repos",
    label: "List Indexed Repositories",
    description: `List all repositories indexed in CodeGraph with their IDs.

IMPORTANT: Call this FIRST when exploring a codebase. You need the repo_id 
(UUID) for other tools to get targeted results. Without repo_id, searches 
may return results from wrong repositories.

Example output:
  REPOS (2 total)
  1  40a30ade-...  pi-server  [PROCESSED] 47/47
  2  453fcbd7-...  local  [PROCESSED] 44/44

The first UUID (40a30ade-...) is the repo_id to pass to other tools.`,
    parameters: Type.Object({}),
    async execute(_toolCallId, _params, _signal, _onUpdate, _ctx) {
      const output = await runCodegraph(["ls"]);
      return {
        content: [{ type: "text", text: output }],
      };
    },
  });

  // ============================================
  // Tool: codegraph_help
  // ============================================
  pi.registerTool({
    name: "codegraph_help",
    label: "CodeGraph Help",
    description: `Quick reference for CodeGraph tools. Use this if you're unsure which 
tool to use or how to get started.

Tool selection guide:

1. codegraph_list_repos - Start here! Get repo IDs
2. codegraph_search - Semantic search: "what does this code do" 
   Example: query="database connection pool"
3. codegraph_lookup - Find symbol: "where is X defined/used"
   Example: term="AuthService"
4. codegraph_grep - Text search: "find pattern Y in files"
   Example: pattern="update_knowledge", glob="*.rs"
5. codegraph_cat - View file analysis + content
   Example: file="src/main.rs", show_content=true

Workflow: list_repos → search/lookup → cat for details`,
    parameters: Type.Object({}),
    async execute(_toolCallId, _params, _signal, _onUpdate, _ctx) {
      return {
        content: [{
          type: "text",
          text: `CodeGraph Tool Guide
==================

1. codegraph_list_repos - List indexed repos (START HERE to get repo_id)
2. codegraph_search - Semantic search across LLM summaries
3. codegraph_lookup - Find keywords/classes/functions by name
4. codegraph_grep - Text pattern search in files
5. codegraph_cat - View file metadata and content

Workflow: list_repos → search/lookup → cat for details

Example workflow for exploring a new repo:
1. codegraph_list_repos() → get repo_id
2. codegraph_search({query: "database", repo_id: "..."}) → find relevant files
3. codegraph_cat({repo_id: "...", file: "src/db.rs", show_content: true}) → view file`,
        }],
      };
    },
  });

  // ============================================
  // Command: /codegraph-stats
  // ============================================
  pi.registerCommand("codegraph-stats", {
    description: "Show CodeGraph statistics",
    handler: async (_args, ctx) => {
      const output = await runCodegraph(["stats"]);
      ctx.ui.notify(output, "info");
    },
  });

  // ============================================
  // Command: /codegraph-ls
  // ============================================
  pi.registerCommand("codegraph-ls", {
    description: "List indexed repositories",
    handler: async (_args, ctx) => {
      const output = await runCodegraph(["ls"]);
      ctx.ui.notify(output, "info");
    },
  });

  // ============================================
  // Tool: codegraph_search
  // ============================================
  pi.registerTool({
    name: "codegraph_search",
    label: "CodeGraph Search",
    description: `Full-text semantic search across the CodeGraph knowledge base. Searches LLM-generated purpose, summary, and business context fields.

BEST FOR: Finding files related to a concept or feature without knowing exact names.

TIPS:
- Always pass repo_id for targeted results (call list_repos first if unknown)
- Use natural language: "database connection pool" not "SQLite implementation"
- Avoid meta queries: "pi-server" won't work, try "REST API" or "web server"

Example queries:
  GOOD: "database connection pool", "authentication middleware", "error handling"
  BAD: "pi-server repository", "how is X implemented" (too meta)

Example result:
  web/service-worker.js  (score: -2.42)
    Service Worker script that implements caching strategies...`,
    parameters: Type.Object({
      query: Type.String({ 
        description: "Search query describing what to find (e.g., 'database connection pool', 'async queue worker')" 
      }),
      limit: Type.Optional(Type.Number({ 
        description: "Maximum number of results to return",
        minimum: 1,
        maximum: 100,
        default: 5
      })),
      repo_id: Type.Optional(Type.String({ 
        description: "Repository ID (UUID). Omit to search all repos. Get IDs via list_repos tool." 
      })),
    }),
    async execute(_toolCallId, params, signal, _onUpdate, _ctx) {
      const args = ["search", JSON.stringify(params.query)];
      if (params.limit) args.push("--limit", String(params.limit));
      if (params.repo_id) args.push("--repo", params.repo_id);
      
      const output = await runCodegraph(args);
      const results = parseSearchOutput(output);
      
      return {
        content: [{ type: "text", text: output }],
        details: { 
          parsed: true,
          results,
          count: results.length,
        },
      };
    },
  });

  // ============================================
  // Tool: codegraph_lookup
  // ============================================
  pi.registerTool({
    name: "codegraph_lookup",
    label: "CodeGraph Lookup",
    description: `Look up code entities (keywords, classes, functions, imports) by name.

BEST FOR: Finding where specific symbols are defined or used in the codebase.

TIPS:
- Always pass repo_id for targeted results
- Works with partial names: "Database" finds "DatabasePool", "DatabaseConfig", etc.
- Returns categorized results: Keywords, Classes, Functions

Example queries:
  term="Database" → finds DatabasePool, DatabaseConfig classes
  term="execute" → finds execute() function definitions
  term="import" → finds import statements`,
    parameters: Type.Object({
      term: Type.String({ 
        description: "Entity name to search for (e.g., 'Database', 'Connection', 'execute')" 
      }),
      repo_id: Type.Optional(Type.String({ 
        description: "Repository ID (UUID). Get IDs via list_repos tool." 
      })),
    }),
    async execute(_toolCallId, params, signal, _onUpdate, _ctx) {
      const args = ["lookup", JSON.stringify(params.term)];
      if (params.repo_id) args.push("--repo", params.repo_id);
      
      const output = await runCodegraph(args);
      
      return {
        content: [{ type: "text", text: output }],
        details: { parsed: true },
      };
    },
  });

  // ============================================
  // Tool: codegraph_grep
  // ============================================
  pi.registerTool({
    name: "codegraph_grep",
    label: "CodeGraph Grep",
    description: `Search for text patterns within files in indexed repositories.
Similar to ripgrep but searches pre-indexed content.

BEST FOR: Finding specific code patterns, function calls, or variable names.

TIPS:
- Always pass repo_id (UUID) for targeted results
- Use glob to filter by file type: '*.go', '**/*.ts', 'src/**/*.rs'
- Case-insensitive search

Example queries:
  pattern="update_knowledge", glob="*.rs" → find update_knowledge in Rust files
  pattern="TODO", glob="*.{ts,js}" → find TODOs in TypeScript/JS`,
    parameters: Type.Object({
      pattern: Type.String({ 
        description: "Text pattern to search for (case-insensitive)" 
      }),
      repo_id: Type.String({ 
        description: "Repository ID (UUID). Get IDs via list_repos tool." 
      }),
      glob: Type.Optional(Type.String({ 
        description: "File glob pattern (e.g., '*.rs', '**/*.ts', 'src/**/*.go')",
        default: "*"
      })),
      show_numbers: Type.Optional(Type.Boolean({ 
        description: "Show line numbers in results",
        default: true
      })),
    }),
    async execute(_toolCallId, params, signal, _onUpdate, _ctx) {
      const args = ["grep", "--repo", params.repo_id];
      if (params.glob) args.push("--glob", params.glob);
      if (params.show_numbers) args.push("--numbers");
      args.push(JSON.stringify(params.pattern));
      
      const output = await runCodegraph(args);
      
      return {
        content: [{ type: "text", text: output }],
        details: { parsed: true },
      };
    },
  });

  // ============================================
  // Tool: codegraph_cat
  // ============================================
  pi.registerTool({
    name: "codegraph_cat",
    label: "CodeGraph Cat",
    description: `View file metadata and LLM-generated analysis from the knowledge base.
Shows purpose, summary, keywords, classes, functions, and optionally file content.

BEST FOR: Understanding what a file does before reading it, or getting context.

OUTPUT INCLUDES:
- Purpose: One-sentence summary of the file
- Summary: Detailed description of functionality
- Keywords: Important terms found in the file
- Classes: Types/classes defined
- Functions: Key functions with their signatures
- Content: Actual file content (if show_content=true)

TIPS:
- Always pass repo_id (UUID) for targeted results
- Use range="1-50" to view specific line ranges
- Use search to highlight specific patterns in content`,
    parameters: Type.Object({
      repo_id: Type.String({ 
        description: "Repository ID (UUID). Get IDs via list_repos tool." 
      }),
      file: Type.String({ 
        description: "File path relative to repository root (e.g., 'src/main.rs', 'Cargo.toml')" 
      }),
      show_content: Type.Optional(Type.Boolean({ 
        description: "Include file content",
        default: false
      })),
      show_numbers: Type.Optional(Type.Boolean({ 
        description: "Show line numbers in content",
        default: true
      })),
      range: Type.Optional(Type.String({ 
        description: "Line range to show (e.g., '1-50' or '100-')" 
      })),
      search: Type.Optional(Type.String({ 
        description: "Search pattern within file content" 
      })),
    }),
    async execute(_toolCallId, params, signal, _onUpdate, _ctx) {
      const args = ["cat", "--repo", params.repo_id, "--file", params.file];
      if (params.show_content) args.push("--content");
      if (params.show_numbers) args.push("--numbers");
      if (params.range) args.push("--range", params.range);
      if (params.search) args.push("--search", params.search);
      
      const output = await runCodegraph(args);
      
      return {
        content: [{ type: "text", text: output }],
        details: { parsed: true },
      };
    },
  });

  // ============================================
  // Command: /codegraph-ingest
  // ============================================
  pi.registerCommand("codegraph-ingest", {
    description: "Index a local directory into CodeGraph (user-only)",
    handler: async (args, ctx) => {
      const path = args || ".";
      ctx.ui.notify(`Indexing ${path}...`, "info");
      const output = await runCodegraph(["ingest", path]);
      ctx.ui.notify(`Ingest complete`, "success");
    },
  });

  // ============================================
  // Command: /codegraph-delete
  // ============================================
  pi.registerCommand("codegraph-delete", {
    description: "Delete a repository from CodeGraph (user-only)",
    handler: async (args, ctx) => {
      if (!args) {
        ctx.ui.notify("Usage: /codegraph-delete <repo-id>", "error");
        return;
      }
      const output = await runCodegraph(["delete", args]);
      ctx.ui.notify(`Deleted repository`, "success");
    },
  });
}
