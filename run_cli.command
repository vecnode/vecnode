#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  echo "[ERROR] cargo not found in PATH."
  echo "Install Rust first: https://rustup.rs/"
  read -r -p "Press Enter to close..." _
  exit 1
fi

echo "[INFO] Building vn CLI..."
if ! cargo build --manifest-path cli/Cargo.toml -p vn; then
  echo "[ERROR] Build failed."
  read -r -p "Press Enter to close..." _
  exit 1
fi

echo "[INFO] Launching vn..."
./cli/target/debug/vn "$@"
STATUS=$?
if [[ $STATUS -ne 0 ]]; then
  echo "[ERROR] vn exited with code $STATUS."
  read -r -p "Press Enter to close..." _
fi
exit $STATUS
