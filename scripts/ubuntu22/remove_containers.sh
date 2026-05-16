#!/usr/bin/env bash
set -euo pipefail

if ! command -v docker >/dev/null 2>&1; then
  echo "[ERROR] Docker is not available or not in PATH"
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "[ERROR] Docker daemon is not running"
  exit 1
fi

echo "[INFO] Stopping all running containers..."
RUNNING_IDS="$(docker ps -q 2>/dev/null || true)"
if [[ -z "${RUNNING_IDS// /}" ]]; then
  echo "[INFO] No running containers to stop."
else
  echo "$RUNNING_IDS" | xargs -r docker stop >/dev/null 2>&1 || true
  echo "[OK] Running containers stopped."
fi

echo "[INFO] Removing all containers..."
ALL_IDS="$(docker ps -aq 2>/dev/null || true)"
if [[ -z "${ALL_IDS// /}" ]]; then
  echo "[INFO] No containers to remove."
else
  echo "$ALL_IDS" | xargs -r docker rm -f >/dev/null 2>&1 || true
  echo "[OK] All containers removed."
fi

exit 0