/**
 * CodeGraph Extension for pi
 * 
 * Provides tools for querying a local code knowledge graph with 5 core tools:
 * - codegraph_list_repos: List indexed repositories
 * - codegraph_search: Full-text semantic search
 * - codegraph_entity: Entity relationships (defines, uses)
 * - codegraph_file: File relationships (deps, dependents) + content viewing
 * - codegraph_grep: Text pattern search
 */

import type { ExtensionAPI } from "@mariozechner/pi-coding-agent";
import { Type } from "typebox";
import { exec } from "node:child_process";
import { promisify } from "node:util";

const execAsync = promisify(exec);

// Path to codegraph binary
const CODEGRAPH_BIN = process.env.CODEGRAPH_BIN || "/data/jbutler/git/jbutlerdev/codegraph/target/release/codegraph";

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

export default function (pi: ExtensionAPI) {
  pi.on("session_start", async (_event, ctx) => {
    ctx.ui.notify("CodeGraph extension loaded", "info");
  });

  // ============================================
  // Tool 1: codegraph_list_repos
  // ============================================
  pi.registerTool({
    name: "codegraph_list_repos",
    label: "List Indexed Repositories",
    description: `List all repositories indexed in CodeGraph with their IDs.

IMPORTANT: Call this FIRST when exploring a codebase. You need the repo_id 
(UUID) for other tools to get targeted results. Without repo_id, searches 
may return results from wrong repositories.

Short IDs work: use "6fb99013" instead of full UUID "6fb99013-7b19-4148-beb7-7d135a7675f8".`,
    parameters: Type.Object({}),
    async execute(_toolCallId, _params, _signal, _onUpdate, _ctx) {
      const output = await runCodegraph(["ls"]);
      return { content: [{ type: "text", text: output }] };
    },
  });

  // ============================================
  // Tool 2: codegraph_search
  // ============================================
  pi.registerTool({
    name: "codegraph_search",
    label: "CodeGraph Search",
    description: `Full-text semantic search across the CodeGraph knowledge base. Searches LLM-generated purpose, summary, and business context fields.

BEST FOR: Finding files related to a concept without knowing exact names.

TIPS:
- Always pass repo_id for targeted results (call list_repos first)
- Use natural language: "database connection pool" not "SQLite struct"
- Avoid meta queries: "pi-server" won't work, try "REST API"

Examples:
  codegraph_search({query: "database pool", repo_id: "..."})
  codegraph_search({query: "authentication middleware", limit: 10})`,
    parameters: Type.Object({
      query: Type.String({ description: "Search query (e.g., 'database connection pool')" }),
      repo_id: Type.Optional(Type.String({ description: "Repository ID. Short IDs work." })),
      limit: Type.Optional(Type.Number({ description: "Max results", minimum: 1, maximum: 100, default: 5 })),
    }),
    async execute(_toolCallId, params, _signal, _onUpdate, _ctx) {
      const args = ["search", JSON.stringify(params.query)];
      if (params.limit) args.push("--limit", String(params.limit));
      if (params.repo_id) args.push("--repo", params.repo_id);
      const output = await runCodegraph(args);
      return { content: [{ type: "text", text: output }] };
    },
  });

  // ============================================
  // Tool 3: codegraph_entity
  // ============================================
  pi.registerTool({
    name: "codegraph_entity",
    label: "CodeGraph Entity Relationships",
    description: `Find where entities are defined and what files reference them.

OPERATIONS:
- "defines": Find where an entity is defined
- "uses": Find files that reference/use an entity  
- "all": Find definition AND all usages in one view

ENTITY TYPES:
- "class": Classes, interfaces, structs, enums, traits
- "function": Functions, methods, procedures
- "module": Imports, modules, packages

TIPS:
- Short repo IDs work: "6fb99013" instead of full UUID
- Search without line numbers: "ConnectionPool" finds "ConnectionPool (~L40-58)"
- Use all=true to see both definition and usages together

EXAMPLES:
  codegraph_entity({operation: "defines", entity_type: "class", name: "ConnectionPool", repo_id: "..."})
  codegraph_entity({operation: "all", entity_type: "class", name: "Database", repo_id: "..."})
  codegraph_entity({operation: "uses", entity_type: "function", name: "validate", repo_id: "..."})`,
    parameters: Type.Object({
      operation: Type.Union([
        Type.Literal("defines"),
        Type.Literal("uses"),
        Type.Literal("all"),
      ], { description: "What to find: 'defines' (location), 'uses' (references), 'all' (both)" }),
      entity_type: Type.Optional(Type.Union([
        Type.Literal("class"),
        Type.Literal("function"),
        Type.Literal("module"),
      ], { description: "Entity type", default: "class" })),
      name: Type.String({ description: "Entity name (e.g., 'ConnectionPool', 'handle_login')" }),
      repo_id: Type.Optional(Type.String({ description: "Repository ID. Short IDs work. Omit for all repos." })),
    }),
    async execute(_toolCallId, params, _signal, _onUpdate, _ctx) {
      const entityType = params.entity_type || "class";
      const cmd = params.operation === "uses" ? "uses" : "defines";
      // entity_type is positional argument, name comes after
      const args = [cmd, entityType, params.name];
      if (params.repo_id) args.push("--repo", params.repo_id);
      if (params.operation === "all") args.push("--all");
      
      const output = await runCodegraph(args);
      return { content: [{ type: "text", text: output }] };
    },
  });

  // ============================================
  // Tool 4: codegraph_file
  // ============================================
  pi.registerTool({
    name: "codegraph_file",
    label: "CodeGraph File Relationships",
    description: `Get file dependencies or dependents, or view file content.

OPERATIONS:
- "deps": Get classes/functions/modules defined and imported by a file
- "dependents": Get files that depend on this file (reverse deps)
- "cat": View file metadata and content

TIPS:
- Short repo IDs work: "6fb99013" instead of full UUID
- For cat: use show_content=true, range="1-50", search="pattern"

EXAMPLES:
  codegraph_file({operation: "deps", repo_id: "...", file: "src/base_fetcher.py"})
  codegraph_file({operation: "dependents", repo_id: "...", file: "src/base_fetcher.py"})
  codegraph_file({operation: "cat", repo_id: "...", file: "src/main.rs", show_content: true})`,
    parameters: Type.Object({
      operation: Type.Union([
        Type.Literal("deps"),
        Type.Literal("dependents"),
        Type.Literal("cat"),
      ], { description: "Operation: 'deps' (what file uses), 'dependents' (what uses file), 'cat' (view)" }),
      repo_id: Type.String({ description: "Repository ID. Short IDs work!" }),
      file: Type.String({ description: "File path within repo (e.g., 'src/base_fetcher.py')" }),
      show_content: Type.Optional(Type.Boolean({ description: "Include file content (cat only)", default: false })),
      show_numbers: Type.Optional(Type.Boolean({ description: "Show line numbers (cat only)", default: true })),
      range: Type.Optional(Type.String({ description: "Line range e.g. '1-50' (cat only)" })),
      search: Type.Optional(Type.String({ description: "Search pattern in content (cat only)" })),
    }),
    async execute(_toolCallId, params, _signal, _onUpdate, _ctx) {
      let args: string[];
      
      if (params.operation === "cat") {
        args = ["cat", "--repo", params.repo_id, "--file", params.file];
        if (params.show_content) args.push("--content");
        if (params.show_numbers) args.push("--numbers");
        if (params.range) args.push("--range", params.range);
        if (params.search) args.push("--search", params.search);
      } else {
        args = [params.operation, "--repo", params.repo_id, "--file", params.file];
      }
      
      const output = await runCodegraph(args);
      return { content: [{ type: "text", text: output }] };
    },
  });

  // ============================================
  // Tool 6: codegraph_top
  // ============================================
  pi.registerTool({
    name: "codegraph_top",
    label: "CodeGraph Top Entities",
    description: `Find the most depended-upon entities (classes, functions, or modules).

BEST FOR: Finding the most important/central entities in a codebase.

ENTITY TYPES:
- "class": Classes, structs, interfaces, enums
- "function": Functions, methods, procedures
- "module": Imports, modules, packages

TIPS:
- Short repo IDs work: "6fb99013" instead of full UUID
- Use limit to control how many results to show (default: 10)

EXAMPLES:
  codegraph_top({entity_type: "class", repo_id: "...", limit: 10})
  codegraph_top({entity_type: "function", repo_id: "...", limit: 5})`,
    parameters: Type.Object({
      entity_type: Type.Optional(Type.Union([
        Type.Literal("class"),
        Type.Literal("function"),
        Type.Literal("module"),
      ], { description: "Entity type", default: "class" })),
      repo_id: Type.Optional(Type.String({ description: "Repository ID. Short IDs work. Omit for all repos." })),
      limit: Type.Optional(Type.Number({ description: "Max results", minimum: 1, maximum: 100, default: 10 })),
    }),
    async execute(_toolCallId, params, _signal, _onUpdate, _ctx) {
      const entityType = params.entity_type || "class";
      const args = ["top", entityType];
      if (params.repo_id) args.push("--repo", params.repo_id);
      if (params.limit && params.limit !== 10) args.push("--limit", String(params.limit));
      const output = await runCodegraph(args);
      return { content: [{ type: "text", text: output }] };
    },
  });

  // ============================================
  // Tool 5: codegraph_grep
  // ============================================
  pi.registerTool({
    name: "codegraph_grep",
    label: "CodeGraph Grep",
    description: `Search for text patterns within files.

BEST FOR: Finding specific code patterns, function calls, or variable names.

TIPS:
- Always pass repo_id
- Use glob to filter: "*.rs", "**/*.ts", "src/**/*.go"
- Case-insensitive search

EXAMPLES:
  codegraph_grep({pattern: "update_knowledge", repo_id: "...", glob: "*.rs"})
  codegraph_grep({pattern: "TODO", repo_id: "...", glob: "*.{ts,js}"})`,
    parameters: Type.Object({
      pattern: Type.String({ description: "Text pattern to search for (case-insensitive)" }),
      repo_id: Type.String({ description: "Repository ID. Short IDs work!" }),
      glob: Type.Optional(Type.String({ description: "File glob pattern", default: "*" })),
      show_numbers: Type.Optional(Type.Boolean({ description: "Show line numbers", default: true })),
    }),
    async execute(_toolCallId, params, _signal, _onUpdate, _ctx) {
      const args = ["grep", "--repo", params.repo_id];
      if (params.glob) args.push("--glob", params.glob);
      if (params.show_numbers) args.push("--numbers");
      args.push(JSON.stringify(params.pattern));
      const output = await runCodegraph(args);
      return { content: [{ type: "text", text: output }] };
    },
  });

  // ============================================
  // Commands (for users)
  // ============================================
  pi.registerCommand("codegraph-stats", {
    description: "Show CodeGraph statistics",
    handler: async (_args, ctx) => {
      const output = await runCodegraph(["stats"]);
      ctx.ui.notify(output, "info");
    },
  });

  pi.registerCommand("codegraph-ls", {
    description: "List indexed repositories",
    handler: async (_args, ctx) => {
      const output = await runCodegraph(["ls"]);
      ctx.ui.notify(output, "info");
    },
  });

  pi.registerCommand("codegraph-ingest", {
    description: "Index a local directory",
    handler: async (args, ctx) => {
      const path = args || ".";
      ctx.ui.notify(`Indexing ${path}...`, "info");
      await runCodegraph(["ingest", path]);
      ctx.ui.notify(`Ingest complete`, "success");
    },
  });

  pi.registerCommand("codegraph-delete", {
    description: "Delete a repository",
    handler: async (args, ctx) => {
      if (!args) {
        ctx.ui.notify("Usage: /codegraph-delete <repo-id>", "error");
        return;
      }
      await runCodegraph(["delete", args]);
      ctx.ui.notify(`Deleted repository`, "success");
    },
  });
}
