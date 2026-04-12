#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# run_silverbullet.sh
# Build and run SilverBullet using Docker from a specified directory.
#
# Usage:
#   ./run_silverbullet.sh
#
# Requirements (Linux):
#   - docker
# ---------------------------------------------------------------------------

clear
echo ""
echo "# ============================"
echo "# SilverBullet Docker Runner"
echo "# ============================"
echo ""

echo "[INFO] Checking for required tools."
if ! command -v docker >/dev/null 2>&1; then
  echo "[ERROR] Docker is not available or not in PATH"
  echo ""
  echo "Docker is required to run this script."
  echo "Please install Docker Engine/Desktop from:"
  echo "  https://docs.docker.com/engine/install/"
  exit 1
fi

DOCKER_VERSION="$(docker --version)"
echo "[OK] ${DOCKER_VERSION}"

if ! docker ps >/dev/null 2>&1; then
  echo "[ERROR] Docker daemon is not running"
  echo ""
  echo "Please start Docker and try again."
  exit 1
fi

echo "[OK] Docker daemon is running."
echo ""

normalize_path_input() {
  local raw="$1"
  local first_char=""
  local last_char=""

  # Trim one pair of matching surrounding quotes if present.
  if (( ${#raw} >= 2 )); then
    first_char="${raw:0:1}"
    last_char="${raw: -1}"
    if [[ "$first_char" == '"' && "$last_char" == '"' ]] || [[ "$first_char" == "'" && "$last_char" == "'" ]]; then
      raw="${raw:1:${#raw}-2}"
    fi
  fi

  # Remove trailing slash for consistent checks.
  raw="${raw%/}"
  printf '%s' "$raw"
}

detect_desktop_dir() {
  local desktop_path=""

  if command -v xdg-user-dir >/dev/null 2>&1; then
    desktop_path="$(xdg-user-dir DESKTOP 2>/dev/null || true)"
  fi

  if [[ -z "$desktop_path" || "$desktop_path" == "$HOME" ]]; then
    desktop_path="$HOME/Desktop"
  fi

  if [[ ! -d "$desktop_path" ]]; then
    desktop_path="$HOME/Desktop"
  fi

  printf '%s' "$desktop_path"
}

while true; do
  echo ""
  read -r -p "Enter path to SilverBullet repository: " SILVERBULLET_PATH
  SILVERBULLET_PATH="$(normalize_path_input "$SILVERBULLET_PATH")"
  if [[ -z "${SILVERBULLET_PATH}" ]]; then
    echo "[ERROR] Path cannot be empty."
    continue
  fi

  if [[ ! -d "${SILVERBULLET_PATH}" ]]; then
    echo "[ERROR] Path does not exist: ${SILVERBULLET_PATH}"
    continue
  fi

  if [[ ! -f "${SILVERBULLET_PATH}/Dockerfile" ]]; then
    echo "[ERROR] This doesn't appear to be a SilverBullet repository."
    echo "[ERROR] Missing: Dockerfile in ${SILVERBULLET_PATH}"
    continue
  fi

  break
done

echo "[OK] Valid SilverBullet repository detected."
echo ""

DESKTOP_DIR="$(detect_desktop_dir)"
DEFAULT_SB_SPACE_PATH="$DESKTOP_DIR/silverbullet"

while true; do
  read -r -p "Enter path to SilverBullet space folder (default '${DEFAULT_SB_SPACE_PATH}'): " SB_SPACE_PATH
  if [[ -z "${SB_SPACE_PATH}" ]]; then
    SB_SPACE_PATH="$DEFAULT_SB_SPACE_PATH"
  fi
  SB_SPACE_PATH="$(normalize_path_input "$SB_SPACE_PATH")"
  if [[ -z "${SB_SPACE_PATH}" ]]; then
    echo "[ERROR] Space folder path cannot be empty."
    continue
  fi

  if [[ ! -d "${SB_SPACE_PATH}" ]]; then
    echo "[INFO] Space folder does not exist, creating it."
    if ! mkdir -p "${SB_SPACE_PATH}"; then
      echo "[ERROR] Failed to create space folder: ${SB_SPACE_PATH}"
      exit 1
    fi
  fi

  break
done

SB_PORT="3000"
read -r -p "Enter host port (default 3000): " INPUT_PORT
if [[ -n "${INPUT_PORT}" ]]; then
  SB_PORT="${INPUT_PORT}"
fi

echo "[INFO] Building Docker image."
echo ""
if ! docker build -t silverbullet:local "${SILVERBULLET_PATH}"; then
  echo "[ERROR] Docker build failed"
  exit 1
fi

echo "[OK] Docker image built successfully."
echo ""

echo "[INFO] Starting SilverBullet container."
echo "[INFO] SilverBullet will be available at http://localhost:${SB_PORT}"
echo ""

docker rm -f silverbullet-local >/dev/null 2>&1 || true

if ! docker run -d --name silverbullet-local -p "${SB_PORT}:3000" --mount type=bind,source="${SB_SPACE_PATH}",target=/space --entrypoint /silverbullet silverbullet:local /space >/dev/null; then
  echo "[ERROR] Docker run failed"
  exit 1
fi

echo "[OK] Container started: silverbullet-local"
echo "[INFO] Open: http://localhost:${SB_PORT}"
echo "[INFO] Data folder: ${SB_SPACE_PATH}"
echo "[INFO] Stop with: docker stop silverbullet-local"
echo "[INFO] Logs with: docker logs -f silverbullet-local"
