# CodeGraph Extension for pi

This extension provides 5 tools for querying a local code knowledge graph powered by CodeGraph.

## Tools (5 total)

### 1. codegraph_list_repos
**Start here!** List all indexed repositories with their IDs.

```typescript
await codegraph_list_repos()
// Returns: UUIDs for all indexed repos
```

### 2. codegraph_search
Full-text semantic search across LLM-generated summaries.

```typescript
await codegraph_search({
  query: "database connection pool",
  limit: 5,
  repo_id: "40a30ade-...",
})
```

### 3. codegraph_entity
Entity relationships - find where code is defined and what uses it.

| Operation | Description |
|-----------|-------------|
| `defines` | Find where entity is defined |
| `uses` | Find files that reference entity |
| `all` | Both definition AND usages |

```typescript
// Where is ConnectionPool defined?
await codegraph_entity({
  operation: "defines",
  entity_type: "class",
  name: "ConnectionPool",
  repo_id: "...",
})

// Definition + all usages together
await codegraph_entity({
  operation: "all",
  entity_type: "class",
  name: "ConnectionPool",
  repo_id: "...",
})

// What files use BaseFetcher?
await codegraph_entity({
  operation: "uses",
  entity_type: "class",
  name: "BaseFetcher",
  repo_id: "...",
})
```

### 4. codegraph_file
File relationships and content viewing.

| Operation | Description |
|-----------|-------------|
| `deps` | What does file define and import |
| `dependents` | What files depend on this file |
| `cat` | View file metadata and content |

```typescript
// What does base_fetcher.py depend on?
await codegraph_file({
  operation: "deps",
  repo_id: "...",
  file: "src/base_fetcher.py",
})

// What depends on base_fetcher.py?
await codegraph_file({
  operation: "dependents",
  repo_id: "...",
  file: "src/base_fetcher.py",
})

// View file content
await codegraph_file({
  operation: "cat",
  repo_id: "...",
  file: "src/main.rs",
  show_content: true,
  range: "1-50",
})
```

### 5. codegraph_grep
Text pattern search within files.

```typescript
await codegraph_grep({
  pattern: "update_knowledge",
  repo_id: "...",
  glob: "*.rs",
})
```

## Recommended Workflow

```
1. codegraph_list_repos()  → Get repo_id
2. codegraph_search()      → Find relevant files by concept
3. codegraph_entity()      → Find where code is defined/used
4. codegraph_file()        → Check file deps or view content
```

## Tips

- **Short repo IDs work**: Use `6fb99013` instead of full UUID
- **Search without line numbers**: `ConnectionPool` finds `ConnectionPool (~L40-58)`
- **Use operation=all**: Shows both definition and usages in one call
