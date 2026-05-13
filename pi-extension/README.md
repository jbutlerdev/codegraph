# CodeGraph pi Extension

A [pi](https://github.com/mariozechner/pi-coding-agent) extension that exposes CodeGraph CLI tools to agents.

## Installation

The extension is auto-discovered when placed in `~/.pi/agent/extensions/`.

To use this extension from the CodeGraph repo:

```bash
ln -sf /data/jbutler/git/jbutlerdev/codegraph/pi-extension ~/.pi/agent/extensions/codegraph
```

## Available Tools (for agents)

| Tool | Description |
|------|-------------|
| `codegraph_search` | Full-text search (LLM summaries) |
| `codegraph_lookup` | Entity lookup (keywords, classes, functions) |
| `codegraph_grep` | Text search in files |
| `codegraph_cat` | View file metadata + content |

## Available Commands (for users)

| Command | Description |
|---------|-------------|
| `/codegraph-stats` | Show indexing statistics |
| `/codegraph-ls` | List indexed repositories |
| `/codegraph-ingest` | Index a directory |
| `/codegraph-delete` | Remove a repository |

## Usage

After loading, the agent can use these tools for code knowledge queries:

```
User: Find files related to database operations
Agent: codegraph_search({ query: "database persistence sqlite" })
       codegraph_lookup({ term: "Database" })
```

## Configuration

The extension looks for the CodeGraph binary at:
```
/data/jbutler/git/jbutlerdev/codegraph/target/release/codegraph
```

## Development

To test changes:
```bash
pi --reload
```

Or run with the extension directly:
```bash
pi -e ./pi-extension/src/index.ts
```
