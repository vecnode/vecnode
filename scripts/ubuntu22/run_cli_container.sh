#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# run_cli_container.sh
# Build and run the vecnode CLI container in interactive mode.
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DOCKERFILE_PATH="$REPO_ROOT/docker/Dockerfile"
BUILD_CONTEXT="$REPO_ROOT"
IMAGE_NAME="vecnode-cli:latest"
CONTAINER_NAME="vecnode-cli-session"

echo ""
echo "# ============================"
echo "# vecnode CLI Container"
echo "# ============================"
echo ""

echo "[INFO] Repository root: $REPO_ROOT"
echo "[INFO] Dockerfile: $DOCKERFILE_PATH"
echo "[INFO] Build context: $BUILD_CONTEXT"
echo "[INFO] Image: $IMAGE_NAME"
echo ""

if ! command -v docker >/dev/null 2>&1; then
  echo "[ERROR] Docker is not available or not in PATH"
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "[ERROR] Docker daemon is not running"
  exit 1
fi

if [[ ! -f "$DOCKERFILE_PATH" ]]; then
  echo "[ERROR] Dockerfile not found: $DOCKERFILE_PATH"
  exit 1
fi

echo "[INFO] Building image..."
docker build -t "$IMAGE_NAME" -f "$DOCKERFILE_PATH" "$BUILD_CONTEXT"

echo ""
echo "[INFO] Starting container in interactive mode..."
echo "[INFO] Container name: $CONTAINER_NAME"
echo "[INFO] Opening shell: /bin/bash"
echo "[INFO] Tools CLI command: bash /app/scripts/tools-cli/ubuntu22/main.sh"
echo "[INFO] vecnode CLI command: bash /app/scripts/ubuntu22/main.sh"
echo ""

# Ensure previous container with same name doesn't block startup.
docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true

exec docker run --rm -it --name "$CONTAINER_NAME" --entrypoint /bin/bash "$IMAGE_NAME"
