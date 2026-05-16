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

echo "[INFO] Removing all Docker images..."
IMAGE_IDS="$(docker images -aq 2>/dev/null || true)"
if [[ -z "${IMAGE_IDS// /}" ]]; then
  echo "[INFO] No images to remove."
else
  echo "$IMAGE_IDS" | xargs -r docker rmi -f >/dev/null 2>&1 || true
  echo "[OK] All images removed."
fi

exit 0