#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# clear_stirling_pdf_image.sh
# Remove the Stirling-PDF Docker image to free disk space (forces a fresh
# download on the next open). The container is removed first if present.
# ---------------------------------------------------------------------------

IMAGE="stirlingtools/stirling-pdf:latest"
CONTAINER="stirling-pdf"

if ! command -v docker >/dev/null 2>&1; then
  echo "[ERROR] Docker is not available or not in PATH."
  exit 1
fi

if docker ps -a --filter "name=^/${CONTAINER}$" --format '{{.Names}}' | grep -qi "${CONTAINER}"; then
  echo "[INFO] Removing container '${CONTAINER}' first..."
  docker rm -f "${CONTAINER}" >/dev/null
fi

if ! docker image inspect "${IMAGE}" >/dev/null 2>&1; then
  echo "[INFO] Image '${IMAGE}' is not present. Nothing to clear."
  exit 0
fi

echo "[INFO] Removing image '${IMAGE}'..."
docker rmi "${IMAGE}" >/dev/null
echo "[OK] Removed image '${IMAGE}'."
exit 0
