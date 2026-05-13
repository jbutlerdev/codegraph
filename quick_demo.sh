#!/bin/bash
# Quick demo - shows CLI structure without requiring API key

cd "$(dirname "$0")"

echo "=========================================="
echo "CodeGraph CLI Structure Demo"
echo "=========================================="
echo ""

# Build if needed
if [ ! -f "target/release/codegraph" ]; then
    echo "Building..."
    cargo build --release -q 2>/dev/null
fi

BIN="./target/release/codegraph"

echo "=== Available Commands ==="
$BIN --help

echo ""
echo "=== Index Command ==="
$BIN index --help

echo ""
echo "=== Search Command ==="
$BIN search --help

echo ""
echo "=== Lookup Command ==="
$BIN lookup --help

echo ""
echo "=== Cat Command ==="
$BIN cat --help

echo ""
echo "=== Grep Command ==="
$BIN grep --help

echo ""
echo "=== Config Commands ==="
$BIN config --help

echo ""
echo "=== Current Configuration ==="
$BIN config ls

echo ""
echo "=========================================="
echo "To configure for your LLM endpoint:"
echo ""
echo "  codegraph config set llm_endpoint http://localhost:8080/v1"
echo "  codegraph config set llm_api_key your-key"
echo "  codegraph config set llm_model your-model"
echo ""
echo "Example for local proxy:"
echo "  codegraph config set llm_endpoint http://bitfrost.botnet:8080/v1"
echo "  codegraph config set llm_api_key bifrost"
echo "  codegraph config set llm_model llamacpp/qwen3.6-27b"
echo ""
echo "To run full demo with indexing:"
echo "  ./demo.sh"
echo "=========================================="
