# CodeGraph

Local code knowledge graph engine - index codebases and search using LLM-powered analysis.

## Features

- **Index GitHub repositories** or local directories
- **LLM-powered file analysis** - extracts purpose, summary, business context
- **Entity extraction** - keywords, classes, functions, modules (internal/external)
- **Full-text search** via SQLite FTS5
- **Diff-aware re-indexing** - only re-analyzes changed files
- **Concurrent ingestion** with configurable parallelism
- **Multi-API support** - OpenAI, Anthropic, OpenAI-compatible endpoints

## Installation

```bash
cargo build --release
ln -sf target/release/codegraph ~/.cargo/bin/codegraph
```

## Configuration

```bash
# View current config
codegraph config ls

# Set LLM endpoint (Anthropic, OpenAI, or OpenAI-compatible)
codegraph config set llm_endpoint http://localhost:8080/anthropic

# Set API type (anthropic, openai, openai-responses)
codegraph config set llm_api_type anthropic

# Set model
codegraph config set llm_model minimax-anthropic/minimax-m2.7-highspeed

# Set API key
codegraph config set llm_api_key your-api-key

# Set concurrency (default: 4)
codegraph config set concurrency 4

# Set max file size before chunking (0 = disabled, default: 12000 tokens)
codegraph config set max_file_tokens 250000
```

Config file: `~/.codegraph/config.toml`

## Quick Start

### Index a repository

```bash
# Local directory
codegraph ingest /path/to/my/project

# GitHub repository
codegraph index https://github.com/user/repo
```

### Search

```bash
# Full-text search
codegraph search "authentication"

# Search in specific repo
codegraph search "retry" --repo <repo-id>

# Lookup entities (keywords, classes, functions)
codegraph lookup "AuthService"

# View file details with content
codegraph cat --repo <repo-id> --file src/auth/login.rs --content
```

### Manage

```bash
# List indexed repos
codegraph ls

# Show statistics
codegraph stats

# Re-index (diff-aware, only changed files)
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
│  │ Scanner │→ │ Analyzer │→ │ LLM Client   │  │
│  └─────────┘  └──────────┘  └──────────────┘  │
│                    ↓                            │
│  ┌─────────────────────────────────────────┐   │
│  │  SQLite + FTS5 (Embedded)               │   │
│  │  - Knowledge (repos)                    │   │
│  │  - Files (purpose, summary, context)     │   │
│  │  - Entities (keywords, classes, fns)    │   │
│  └─────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

## Supported LLM APIs

| API Type | Endpoint Format | Notes |
|----------|-----------------|-------|
| `openai` | `/v1/chat/completions` | OpenAI-compatible APIs |
| `anthropic` | `/v1/messages` | Anthropic Messages API |
| `openai-responses` | `/v1/chat/completions` | OpenAI Responses API (future) |

## CLI Commands

| Command | Description |
|---------|-------------|
| `index <url>` | Index GitHub repository |
| `ingest <path>` | Index local directory |
| `ls` | List indexed repositories |
| `search <query>` | Full-text search |
| `lookup <term>` | Entity lookup |
| `cat --repo <id> --file <path>` | Show file details |
| `grep <pattern>` | Grep files in repo |
| `defines <entity>` | Find entity definitions |
| `uses <entity>` | Find entity references |
| `stats` | Show statistics |
| `pull <id>` | Re-index repository |
| `delete <id>` | Delete repository |
| `config get <key>` | Get config value |
| `config set <key> <value>` | Set config value |
| `config ls` | List all config |

## Database

Stored at `~/.codegraph/codegraph.db` (SQLite with WAL mode)

Tables:
- `knowledge` - repositories
- `files` - file metadata and analysis
- `keywords` - extracted keywords
- `classes` - extracted classes
- `functions` - extracted functions
- `modules` - extracted modules
- `file_*` - relationship edges

## License

AGPL-3.0-or-later
