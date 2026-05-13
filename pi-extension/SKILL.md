# CodeGraph Extension for pi

This extension provides tools for querying a local code knowledge graph powered by CodeGraph.

## Overview

CodeGraph indexes code repositories locally using:
- **SQLite + FTS5** for full-text search
- **LLM analysis** for purpose, summary, and entity extraction
- **SHA256 diff** to only re-index changed files

## Tools

### codegraph_list_repos
**Start here!** List all indexed repositories with their IDs.

```typescript
await codegraph_list_repos()
// Returns: UUIDs for all indexed repos
```

### codegraph_help
Quick reference guide for all CodeGraph tools.

```typescript
await codegraph_help()
```

### codegraph_search
Full-text semantic search across LLM-generated summaries.

```typescript
await codegraph_search({
  query: "database connection pool",
  limit: 5,
  repo_id: "40a30ade-...",
})
```

### codegraph_lookup
Find code entities (keywords, classes, functions) by name.

```typescript
await codegraph_lookup({
  term: "Database",
  repo_id: "40a30ade-...",
})
```

### codegraph_grep
Search for text patterns within files.

```typescript
await codegraph_grep({
  pattern: "update_knowledge",
  repo_id: "40a30ade-...",
  glob: "*.rs",
})
```

### codegraph_cat
View file metadata, LLM analysis, and content.

```typescript
await codegraph_cat({
  repo_id: "40a30ade-...",
  file: "src/main.rs",
  show_content: true,
  range: "1-50",
})
```

## Recommended Workflow

```
1. codegraph_list_repos()  → Get repo_id (UUID)
2. codegraph_search()      → Find relevant files by concept
3. codegraph_lookup()      → Find specific symbols
4. codegraph_cat()         → View file details + content
```

## Tips

- **Always pass `repo_id`** - Without it, searches may return results from wrong repos
- **Use natural language for search** - "database pool" not "SQLite struct"
- **Avoid meta queries** - "pi-server" won't work, try "REST API" or "web server"
- **Get repo IDs first** - Call `list_repos` at the start of any new codebase exploration

## Commands (for users)

| Command | Description |
|---------|-------------|
| `/codegraph-stats` | Show indexing statistics |
| `/codegraph-ls` | List indexed repositories |
| `/codegraph-ingest` | Index a directory |
| `/codegraph-delete` | Remove a repository |

## Configuration

The extension expects CodeGraph at:
```
/data/jbutler/git/jbutlerdev/codegraph/target/release/codegraph
```

Set `CODEGRAPH_BIN` environment variable or modify the extension to change this path.
