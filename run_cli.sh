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
if ! cargo build --manifest-path cli/Cargo.toml -p vn; then
  echo "[ERROR] Build failed."
  exit 1
fi

if [[ ! -x ./cli/target/debug/vn ]]; then
  echo "[ERROR] Binary not found: ./cli/target/debug/vn"
  exit 1
fi

echo "[INFO] Launching vn..."
set +e
./cli/target/debug/vn "$@"
VN_EXIT=$?
set -e

if [[ "$VN_EXIT" -ne 0 ]]; then
  echo "[ERROR] vn exited with code $VN_EXIT."
fi

exit "$VN_EXIT"
