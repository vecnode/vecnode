#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# stop_media_downloader.sh
# Stop the media-downloader container. It holds no state, so this just stops
# it; reopen rebuilds and runs it fresh.
# ---------------------------------------------------------------------------

CONTAINER="media-downloader"

if ! command -v docker >/dev/null 2>&1; then
  echo "[ERROR] Docker is not available or not in PATH."
  exit 1
fi

if ! docker ps -aq --filter "name=^/${CONTAINER}$" | grep -q .; then
  echo "[INFO] No '${CONTAINER}' container exists. Nothing to stop."
  exit 0
fi

echo "[INFO] Stopping '${CONTAINER}'..."
docker stop "${CONTAINER}" >/dev/null
echo "[OK] Stopped '${CONTAINER}'."
exit 0
