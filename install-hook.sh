#!/usr/bin/env bash
# Install pre-commit hooks for cct
# Usage: bash install-hook.sh
set -euo pipefail

echo "=== cct pre-commit hook installer ==="

# Check dependencies
command -v pre-commit >/dev/null 2>&1 || { echo "ERROR: pre-commit not found. Install: pip install pre-commit"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "ERROR: cargo not found. Install Rust toolchain first."; exit 1; }
command -v rustfmt >/dev/null 2>&1 || { echo "ERROR: rustfmt not found. Run: rustup component add rustfmt"; exit 1; }
cargo clippy --version >/dev/null 2>&1 || { echo "ERROR: clippy not found. Run: rustup component add clippy"; exit 1; }

# Install hooks
pre-commit install --overwrite
pre-commit install --hook-type pre-push --overwrite

echo ""
echo "=== Hooks installed ==="
echo "  pre-commit: trailing-whitespace, end-of-file-fixer, check-yaml, check-toml,"
echo "              check-merge-conflict, cargo fmt, cargo clippy"
echo "  pre-push:   cargo test"
echo ""
echo "Run 'pre-commit run --all-files' to verify setup."
