#!/usr/bin/env bash
set -euo pipefail

if ! command -v docker >/dev/null 2>&1; then
  echo "[ERROR] Docker is not available or not in PATH"
  exit 1
fi

docker ps

container_count="$(docker ps -aq | sed '/^$/d' | wc -l | tr -d ' ')"
image_count="$(docker images -aq | sed '/^$/d' | wc -l | tr -d ' ')"

echo "Containers: ${container_count}"
echo "Images: ${image_count}"
