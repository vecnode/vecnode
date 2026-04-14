#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# run_silverbullet.sh
# Run SilverBullet using Docker latest image.
#
# Usage:
#   ./run_silverbullet.sh
#
# Requirements (Linux):
#   - docker
# ---------------------------------------------------------------------------

# ---------------------------------------------------------------------------
# DOCKER CHECK & SETUP
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

# ---------------------------------------------------------------------------
# SILVERBULLET SPACE SETUP
# ---------------------------------------------------------------------------

SB_SPACE_PATH="$HOME/silverbullet-space"

if [[ ! -d "${SB_SPACE_PATH}" ]]; then
  echo "[INFO] Space folder does not exist, creating it."
  if ! mkdir -p "${SB_SPACE_PATH}"; then
    echo "[ERROR] Failed to create space folder: ${SB_SPACE_PATH}"
    exit 1
  fi
  echo "[OK] Created: ${SB_SPACE_PATH}"
else
  echo "[OK] Space folder exists: ${SB_SPACE_PATH}"
fi

echo ""

# ---------------------------------------------------------------------------
# OPTIONAL BACKUP SPACE FOLDER
# ---------------------------------------------------------------------------

while true; do
  echo ""
  read -r -p "Do you want to backup the space folder elsewhere? (y/n): " BACKUP_CHOICE

  if [[ "$BACKUP_CHOICE" == "y" || "$BACKUP_CHOICE" == "Y" || "$BACKUP_CHOICE" == "yes" || "$BACKUP_CHOICE" == "YES" ]]; then
    while true; do
      echo ""
      read -r -p "Enter backup destination folder path (default: ${HOME}/Desktop): " BACKUP_BASE_PATH

      if [[ -z "${BACKUP_BASE_PATH}" ]]; then
        BACKUP_BASE_PATH="${HOME}/Desktop"
      fi

      BACKUP_BASE_PATH="${BACKUP_BASE_PATH/#\~/$HOME}"

      if [[ ! -d "${BACKUP_BASE_PATH}" ]]; then
        echo "[ERROR] Path does not exist: ${BACKUP_BASE_PATH}"
        continue
      fi

      BACKUP_TS="$(date '+%Y%m%d-%H%M%S')"
      BACKUP_TARGET="${BACKUP_BASE_PATH}/silverbullet-space-backup-${BACKUP_TS}"

      if ! mkdir -p "${BACKUP_TARGET}"; then
        echo "[ERROR] Failed to create backup folder: ${BACKUP_TARGET}"
        continue
      fi

      echo "[INFO] Backing up space folder to: ${BACKUP_TARGET}"
      if ! cp -a "${SB_SPACE_PATH}/." "${BACKUP_TARGET}/"; then
        echo "[ERROR] Backup failed."
        exit 1
      fi

      echo "[OK] Backup completed successfully."
      break
    done
    break
  elif [[ "$BACKUP_CHOICE" == "n" || "$BACKUP_CHOICE" == "N" || "$BACKUP_CHOICE" == "no" || "$BACKUP_CHOICE" == "NO" ]]; then
    break
  else
    echo "[ERROR] Invalid choice. Please enter 'y' or 'n'."
  fi
done

echo ""

# ---------------------------------------------------------------------------
# OPTIONAL SYNC FROM ANOTHER FOLDER
# ---------------------------------------------------------------------------

while true; do
  echo ""
  read -r -p "Do you want to sync markdown files from another folder? (y/n): " SYNC_CHOICE
  
  if [[ "$SYNC_CHOICE" == "y" || "$SYNC_CHOICE" == "Y" ]]; then
    while true; do
      echo ""
      read -r -p "Enter path to source markdown folder: " SOURCE_PATH
      
      # Expand ~ to home directory
      SOURCE_PATH="${SOURCE_PATH/#\~/$HOME}"
      
      if [[ -z "${SOURCE_PATH}" ]]; then
        echo "[ERROR] Path cannot be empty."
        continue
      fi
      
      if [[ ! -d "${SOURCE_PATH}" ]]; then
        echo "[ERROR] Path does not exist: ${SOURCE_PATH}"
        continue
      fi
      
      echo "[INFO] Syncing markdown files from: ${SOURCE_PATH}"
      
      # Copy all markdown files (.md) from source to destination
      if cp "${SOURCE_PATH}"/*.md "${SB_SPACE_PATH}/" 2>/dev/null; then
        echo "[OK] Markdown files synced successfully."
      else
        echo "[WARNING] No markdown files found to sync, or sync encountered an issue."
      fi
      
      break
    done
    break
  elif [[ "$SYNC_CHOICE" == "n" || "$SYNC_CHOICE" == "N" ]]; then
    echo "[INFO] Skipping sync."
    break
  else
    echo "[ERROR] Invalid choice. Please enter 'y' or 'n'."
  fi
done

echo ""

# ---------------------------------------------------------------------------
# DOCKER CONTAINER SETUP & RUN
# ---------------------------------------------------------------------------

echo "[INFO] Stopping any existing SilverBullet container."
docker rm -f silverbullet >/dev/null 2>&1 || true

echo "[INFO] Starting SilverBullet container from latest image."
echo "[INFO] SilverBullet will be available at http://localhost:3000"
echo ""

if ! docker run -d --rm \
  --name silverbullet \
  -p 3000:3000 \
  -v "${SB_SPACE_PATH}":/space \
  -e SB_USER="user:password" \
  ghcr.io/silverbulletmd/silverbullet:latest >/dev/null; then
  echo "[ERROR] Docker run failed"
  exit 1
fi

echo "[OK] Container started: silverbullet"
echo "[INFO] Open: http://localhost:3000"
echo "[INFO] Username: user"
echo "[INFO] Password: password"
echo "[INFO] Data folder: ${SB_SPACE_PATH}"
echo "[INFO] Stop with: docker stop silverbullet"
echo "[INFO] Logs with: docker logs -f silverbullet"
