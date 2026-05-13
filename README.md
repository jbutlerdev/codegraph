# CodeGraph

Local code knowledge graph engine - a Rust reimplementation of ByteBell's core functionality.

## Features

- **Index GitHub repositories** or local directories
- **LLM-powered file analysis** via OpenRouter (purpose, summary, business context)
- **Full-text search** across purpose, summary, context, paths, keywords, classes, functions
- **Entity extraction** - keywords, classes, functions, imports
- **Diff-aware re-indexing** - only re-analyzes changed files
- **Embedded storage** - SQLite + FTS5 (no external databases)

## Quick Start

### Configure

```bash
codegraph config set openrouter-api-key sk-or-...
codegraph config set openrouter-model anthropic/claude-sonnet-4.6
```

### Index a repository

```bash
# GitHub repository
codegraph index https://github.com/anthropics/claude-code

# Local directory
codegraph ingest /path/to/my/project
```

### Search

```bash
# Full-text search
codegraph search "authentication"

# Search in specific repo
codegraph search "retry policy" --repo <repo-id>

# Lookup entities (keywords, classes, functions)
codegraph lookup "AuthService"

# View file details
codegraph cat --repo <repo-id> --file src/auth/login.rs
```

### Manage

```bash
# List indexed repos
codegraph ls

# Show statistics
codegraph stats

# Re-index (diff-aware)
codegraph pull <repo-id>

# Delete a repo
codegraph delete <repo-id>
```

## Architecture

```
┌─────────────────────────────────────────────────┐
│  CLI (clap)                                     │
├─────────────────────────────────────────────────┤
│  Ingest Pipeline                                │
│  ┌─────────┐  ┌──────────┐  ┌──────────────┐  │
│  │ Scanner │→ │ Analyzer │→ │ LLM (OpenRouter)│ │
│  └─────────┘  └──────────┘  └──────────────┘  │
│                    ↓                            │
│  ┌─────────────────────────────────────────┐   │
│  │  SQLite + FTS5 (Embedded)               │   │
│  │  - Knowledge (repos)                    │   │
│  │  - Files (purpose, summary, context)   │   │
│  │  - Entities (keywords, classes, fns)  │   │
│  └─────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

## Storage

Replaces ByteBell's Neo4j + MongoDB + Redis stack with embedded SQLite:

| ByteBell | CodeGraph |
|----------|-----------|
| Neo4j | SQLite + FTS5 |
| MongoDB | SQLite (content/metadata) |
| Redis/BullMQ | sled (queue) |
| MCP Server | CLI |

## CLI Commands

| Command | Description |
|---------|-------------|
| `index <url>` | Index GitHub repository |
| `ingest <path>` | Index local directory |
| `ls` | List indexed repositories |
| `search <query>` | Full-text search |
| `lookup <term>` | Entity lookup (keywords, classes, functions) |
| `cat <file>` | Show file metadata |
| `stats` | Show statistics |
| `pull <id>` | Re-index repository |
| `delete <id>` | Delete repository |
| `config` | Configuration management |

## Configuration

Config file: `~/.codegraph/config.toml`

```toml
openrouter-api-key = "sk-or-..."
openrouter-model = "anthropic/claude-sonnet-4.6"
concurrency = 4
log-level = "info"
```

## Installation

```bash
cargo install --path .
```

## License

AGPL-3.0-or-later
