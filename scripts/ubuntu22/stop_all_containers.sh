#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# stop_all_containers.sh
# Stop every running Docker container (without removing them).
# ---------------------------------------------------------------------------

if ! command -v docker >/dev/null 2>&1; then
  echo "[ERROR] Docker is not available or not in PATH"
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "[ERROR] Docker daemon is not running"
  exit 1
fi

RUNNING="$(docker ps -q)"
if [[ -z "$RUNNING" ]]; then
  echo "[INFO] No running containers to stop."
  exit 0
fi

echo "[INFO] Stopping all running containers..."
docker stop $RUNNING >/dev/null
echo "[OK] All running containers stopped."
exit 0
