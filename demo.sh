#!/bin/bash
# Demo script for CodeGraph - indexes this repo and shows functionality
#
# Prerequisites:
# - A running LLM endpoint (e.g., local Ollama, text-gen-webui, etc.)
# - Configure with:
#   codegraph config set llm_endpoint http://localhost:8080/v1
#   codegraph config set llm_api_key your-api-key
#   codegraph config set llm_model llamacpp/qwen3.6-27b

set -e

cd "$(dirname "$0")"

echo "=========================================="
echo "CodeGraph Demo"
echo "=========================================="
echo ""

# Build release if not exists
if [ ! -f "target/release/codegraph" ]; then
    echo "Building codegraph..."
    cargo build --release -q
fi

BIN="./target/release/codegraph"

echo "=========================================="
echo "Step 1: Configure LLM Endpoint"
echo "=========================================="
echo ""

echo "Current configuration:"
$BIN config ls

echo ""
echo "Enter your LLM endpoint (e.g., http://localhost:8080/v1)"
read -p "LLM Endpoint [http://bitfrost.botnet:8080/v1]: " ENDPOINT
ENDPOINT=${ENDPOINT:-http://bitfrost.botnet:8080/v1}

echo "Enter your LLM API key"
read -s -p "API Key [bifrost]: " API_KEY
API_KEY=${API_KEY:-bifrost}
echo ""

echo "Enter your model name"
read -p "Model [llamacpp/qwen3.6-27b]: " MODEL
MODEL=${MODEL:-llamacpp/qwen3.6-27b}

$BIN config set llm_endpoint "$ENDPOINT"
$BIN config set llm_api_key "$API_KEY"
$BIN config set llm_model "$MODEL"

echo ""
echo "Configuration saved."
echo ""

REPO_ID=$(basename "$(pwd)")

echo "=========================================="
echo "Step 2: List current repos"
echo "=========================================="
echo ""

$BIN ls

echo ""
echo "=========================================="
echo "Step 3: Index this repository"
echo "=========================================="
echo ""

echo "Indexing local directory..."
$BIN ingest "$(pwd)"
echo ""
echo "Note: Full indexing requires LLM calls for each file."
echo "This may take a few minutes depending on repo size."
echo ""
echo "Showing current state:"
$BIN ls

echo ""
echo "=========================================="
echo "Step 4: Search functionality"
echo "=========================================="
echo ""

echo "--- Search for 'database' ---"
$BIN search database --limit 5

echo ""
echo "--- Search for 'search' ---"
$BIN search search --limit 5

echo ""
echo "--- Search with JSON output ---"
$BIN search cli --limit 3 --json

echo ""
echo "=========================================="
echo "Step 5: Lookup (entity search)"
echo "=========================================="
echo ""

echo "--- Lookup 'database' ---"
$BIN lookup database

echo ""
echo "--- Lookup 'config' ---"
$BIN lookup config

echo ""
echo "=========================================="
echo "Step 6: Cat (file details)"
echo "=========================================="
echo ""

echo "--- Get metadata for a key file ---"
$BIN cat --repo "$REPO_ID" --file "src/main.rs" 2>/dev/null || echo "Run 'codegraph ingest $(pwd)' first to index"

echo ""
echo "--- With content (first 20 lines) ---"
$BIN cat --repo "$REPO_ID" --file "src/main.rs" --content --range "1-20" --numbers 2>/dev/null || echo "Run 'codegraph ingest $(pwd)' first"

echo ""
echo "=========================================="
echo "Step 7: Grep (bulk file search)"
echo "=========================================="
echo ""

echo "Searching for 'fn main' in Rust files..."
$BIN grep --repo "$REPO_ID" "fn main" --glob "*.rs" --numbers 2>/dev/null || echo "Run 'codegraph ingest $(pwd)' first"

echo ""
echo "Searching for 'Database' in src/ directory..."
$BIN grep --repo "$REPO_ID" "Database" --glob "src/**" --numbers 2>/dev/null || echo "Run 'codegraph ingest $(pwd)' first"

echo ""
echo "=========================================="
echo "Step 8: Statistics"
echo "=========================================="
echo ""

$BIN stats

echo ""
echo "=========================================="
echo "Demo complete!"
echo "=========================================="
echo ""
echo "Configuration keys:"
echo "  llm_endpoint   - API endpoint URL"
echo "  llm_api_key    - API key"
echo "  llm_model      - Model name"
echo ""
echo "Commands:"
echo "  codegraph ls              # List indexed repos"
echo "  codegraph search <term>   # Full-text search"
echo "  codegraph lookup <term>   # Entity lookup"
echo "  codegraph cat --repo <id> --file <path> --content  # Show file"
echo "  codegraph grep --repo <id> <pattern> --glob <files>  # Grep files"
