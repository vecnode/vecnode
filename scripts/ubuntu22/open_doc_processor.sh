#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# open_doc_processor.sh
# Build and run the doc-processor container (image source: docker/media-processor).
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DOCKERFILE_PATH="$REPO_ROOT/docker/media-processor/Dockerfile"
BUILD_CONTEXT="$REPO_ROOT"
IMAGE_NAME="vecnode-doc-processor:latest"
CONTAINER_NAME="vecnode-doc-processor"
UI_PORT="8085"
API_PORT="8086"
HOST_DESKTOP_HOST="${HOME}/Desktop"
HOST_DESKTOP_CONTAINER="/host/Desktop"

clear
echo "[INFO] Repository root: $REPO_ROOT"
echo "[INFO] Dockerfile: $DOCKERFILE_PATH"
echo "[INFO] Image: $IMAGE_NAME"
echo "[INFO] Container: $CONTAINER_NAME"
echo "[INFO] Host Desktop: $HOST_DESKTOP_HOST"
echo ""

if ! command -v docker >/dev/null 2>&1; then
  echo "[ERROR] Docker is not available or not in PATH"
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "[ERROR] Docker daemon is not running"
  echo "[INFO] Start Docker first, then retry."
  exit 1
fi

if [[ ! -f "$DOCKERFILE_PATH" ]]; then
  echo "[ERROR] Dockerfile not found: $DOCKERFILE_PATH"
  exit 1
fi

mkdir -p "$HOST_DESKTOP_HOST"

echo "[INFO] Building media-processor image..."
docker build -t "$IMAGE_NAME" -f "$DOCKERFILE_PATH" "$BUILD_CONTEXT"

echo ""
echo "[INFO] Removing previous container if present..."
docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true

echo "[INFO] Starting doc-processor container..."
docker run -d --rm --name "$CONTAINER_NAME" -p "$UI_PORT":8085 -p "$API_PORT":8086 -e HOST_DESKTOP_DIR="$HOST_DESKTOP_CONTAINER" -v "$HOST_DESKTOP_HOST":"$HOST_DESKTOP_CONTAINER" "$IMAGE_NAME" >/dev/null

echo "[INFO] Waiting for API health endpoint..."
READY=0
for _ in $(seq 1 20); do
  if command -v curl >/dev/null 2>&1 && curl --silent --fail "http://localhost:$API_PORT/health" >/dev/null 2>&1; then
    READY=1
    break
  fi
  sleep 1
done

if [[ "$READY" -eq 1 ]]; then
  echo "[OK] media-processor is ready."
else
  echo "[WARNING] API health check did not pass in time. Container may still be starting."
fi

echo "[INFO] UI:  http://localhost:$UI_PORT"
echo "[INFO] API: http://localhost:$API_PORT"
echo "[INFO] Output folder base: $HOST_DESKTOP_HOST"
echo "[INFO] Logs: docker logs -f $CONTAINER_NAME"
echo "[INFO] Stop: docker stop $CONTAINER_NAME"

if command -v xdg-open >/dev/null 2>&1; then
  xdg-open "http://localhost:$UI_PORT" >/dev/null 2>&1 || true
fi

exit 0