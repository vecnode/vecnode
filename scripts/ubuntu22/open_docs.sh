#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DOCKERFILE_PATH="$REPO_ROOT/docs/Dockerfile"
BUILD_CONTEXT="$REPO_ROOT/docs"
IMAGE_NAME="vecnode-docs:latest"
CONTAINER_NAME="vecnode-docs"
DOCS_PORT=""

echo "[INFO] Repository root: $REPO_ROOT"
echo "[INFO] Docs Dockerfile: $DOCKERFILE_PATH"
echo "[INFO] Build context: $BUILD_CONTEXT"

if ! command -v docker >/dev/null 2>&1; then
  echo "[ERROR] Docker is not available or not in PATH"
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "[ERROR] Docker daemon is not running"
  exit 1
fi

if [[ ! -f "$DOCKERFILE_PATH" ]]; then
  echo "[ERROR] Docs Dockerfile not found: $DOCKERFILE_PATH"
  exit 1
fi

echo "[INFO] Building docs image..."
docker build -t "$IMAGE_NAME" -f "$DOCKERFILE_PATH" "$BUILD_CONTEXT"

echo "[INFO] Recreating docs container: $CONTAINER_NAME"
docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true

if docker run -d --name "$CONTAINER_NAME" -p 127.0.0.1:3000:3000 "$IMAGE_NAME" >/dev/null 2>&1; then
  DOCS_PORT="3000"
else
  echo "[WARNING] Local port 3000 is unavailable. Falling back to another localhost port."
  docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true

  docker run -d --name "$CONTAINER_NAME" -P "$IMAGE_NAME" >/dev/null

  PORT_LINE="$(docker port "$CONTAINER_NAME" 3000/tcp 2>/dev/null | head -n1)"
  DOCS_PORT="${PORT_LINE##*:}"

  if [[ -z "$DOCS_PORT" ]]; then
    echo "[ERROR] Could not determine mapped docs port."
    exit 1
  fi
fi

DOCS_URL="http://localhost:$DOCS_PORT"

echo "[INFO] Docs container started."
echo "[INFO] Open docs at: $DOCS_URL"

if command -v xdg-open >/dev/null 2>&1; then
  xdg-open "$DOCS_URL" >/dev/null 2>&1 || true
  echo "[INFO] Opening $DOCS_URL"
fi

exit 0