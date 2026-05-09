#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  echo "[ERROR] cargo not found in PATH."
  echo "Install Rust first: https://rustup.rs/"
  exit 1
fi

echo "[INFO] Building vn CLI..."
cargo build --manifest-path cli/Cargo.toml -p vn

echo "[INFO] Launching vn..."
exec ./cli/target/debug/vn "$@"
