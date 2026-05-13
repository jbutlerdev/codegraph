# CodeGraph pi Extension

A [pi](https://github.com/mariozechner/pi-coding-agent) extension that exposes CodeGraph CLI tools to agents.

## Installation

The extension is auto-discovered when placed in `~/.pi/agent/extensions/`.

```bash
ln -sf /data/jbutler/git/jbutlerdev/codegraph/pi-extension ~/.pi/agent/extensions/codegraph
```

## Available Tools (5 total)

| Tool | Purpose |
|------|---------|
| `codegraph_list_repos` | List indexed repos, get repo_id |
| `codegraph_search` | Semantic search across code |
| `codegraph_entity` | Find where code is defined/used |
| `codegraph_file` | File deps, dependents, or content |
| `codegraph_grep` | Text pattern search |

## Usage Examples

```typescript
// Start: get repo ID
codegraph_list_repos()

// Search for relevant files
codegraph_search({query: "database pool", repo_id: "..."})

// Find where a class is defined
codegraph_entity({
  operation: "defines",
  entity_type: "class",
  name: "ConnectionPool",
  repo_id: "..."
})

// Get both definition and usages
codegraph_entity({
  operation: "all",
  entity_type: "class",
  name: "ConnectionPool",
  repo_id: "..."
})

// Check what a file depends on
codegraph_file({
  operation: "deps",
  repo_id: "...",
  file: "src/base_fetcher.py"
})

// View file content
codegraph_file({
  operation: "cat",
  repo_id: "...",
  file: "src/main.rs",
  show_content: true
})
```

## Tips

- **Short repo IDs work**: `6fb99013` instead of full UUID
- **Search without line numbers**: `ConnectionPool` finds `ConnectionPool (~L40-58)`
- **Use operation=all**: Shows both definition and usages together

## Configuration

```
/data/jbutler/git/jbutlerdev/codegraph/target/release/codegraph
```

Set `CODEGRAPH_BIN` env var to change this path.
