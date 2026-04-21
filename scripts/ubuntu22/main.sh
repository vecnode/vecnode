#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# main.sh
# Entry point for vecnode CLI
#
# Usage:
#   ./main.sh
#
# Requirements (Linux):
#   - git
#   - curl
#   - jq
# ---------------------------------------------------------------------------

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ---------------------------------------------------------------------------
# HEADER & REQUIREMENTS CHECK
# ---------------------------------------------------------------------------

clear
echo ""
echo "# ============================"
echo "# vecnode"
echo "# Linux CLI"
echo "# ============================"
echo ""

for cmd in git curl jq; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[ERROR] Required command not found: $cmd"
    echo "Please install: git, curl, and jq"
    exit 1
  fi
done
# ---------------------------------------------------------------------------
# MAIN MENU - CHOOSE OPERATION
# ---------------------------------------------------------------------------

while true; do
  echo ""
  echo "What would you like to do?"
  echo "  1 = Docker"
  echo "  2 = GitHub"
  echo "  3 = Silverbullet"
  echo "  4 = Settings"
  echo "  5 = Quit"
  echo ""
  read -r -p "Enter your choice (1, 2, 3, 4, or 5): " MAIN_CHOICE
  clear

  if [[ "$MAIN_CHOICE" == "1" ]]; then
    echo ""

    if ! command -v docker >/dev/null 2>&1; then
      echo "[ERROR] Docker is not available or not in PATH"
      echo "Please install Docker Engine/Desktop from:"
      echo "  https://docs.docker.com/engine/install/"
      exit 1
    fi

    while true; do
      echo "What would you like to do?"
      echo "  1 = Clear Containers and Images"
      echo "  2 = Start CLI Container"
      echo "  3 = Menu"
      echo "  4 = Quit"
      echo ""
      read -r -p "Enter your choice (1, 2, 3, or 4): " DOCKER_CHOICE
      clear

      if [[ "$DOCKER_CHOICE" == "1" ]]; then
        echo ""
        sudo docker stop $(sudo docker ps -aq) 2>/dev/null || echo "No containers to stop"
        sudo docker rm -f $(sudo docker ps -aq) 2>/dev/null || echo "No containers to remove"
        sudo docker rmi -f $(sudo docker images -aq) 2>/dev/null || echo "No images to remove"
        echo ""
        continue
      fi

      if [[ "$DOCKER_CHOICE" == "2" ]]; then
        echo ""
        if command -v gnome-terminal >/dev/null 2>&1; then
          gnome-terminal -- bash -lc "bash \"$SCRIPT_DIR/run_cli_container.sh\""
          echo "[INFO] Opened CLI container in gnome-terminal."
        elif command -v konsole >/dev/null 2>&1; then
          konsole -e bash -lc "bash \"$SCRIPT_DIR/run_cli_container.sh\""
          echo "[INFO] Opened CLI container in konsole."
        elif command -v xfce4-terminal >/dev/null 2>&1; then
          xfce4-terminal --command="bash -lc 'bash \"$SCRIPT_DIR/run_cli_container.sh\"'"
          echo "[INFO] Opened CLI container in xfce4-terminal."
        elif command -v x-terminal-emulator >/dev/null 2>&1; then
          x-terminal-emulator -e bash -lc "bash \"$SCRIPT_DIR/run_cli_container.sh\""
          echo "[INFO] Opened CLI container in x-terminal-emulator."
        elif command -v xterm >/dev/null 2>&1; then
          xterm -e bash -lc "bash \"$SCRIPT_DIR/run_cli_container.sh\""
          echo "[INFO] Opened CLI container in xterm."
        else
          echo "[WARNING] No GUI terminal detected. Running in current terminal."
          bash "$SCRIPT_DIR/run_cli_container.sh"
        fi
        echo ""
        continue
      fi

      if [[ "$DOCKER_CHOICE" == "3" ]]; then
        echo ""
        break
      fi

      if [[ "$DOCKER_CHOICE" == "4" ]]; then
        echo ""
        echo "[INFO] Exiting."
        exit 0
      fi

      echo "[ERROR] Invalid choice. Please enter 1, 2, 3, or 4."
      echo ""
    done
    continue
  fi

  if [[ "$MAIN_CHOICE" == "2" ]]; then
    echo ""
    while true; do
      echo "What would you like to do?"
      echo "  1 = Backup GitHub"
      echo "  2 = Menu"
      echo "  3 = Quit"
      echo ""
      read -r -p "Enter your choice (1, 2, or 3): " GITHUB_MENU_CHOICE
      clear

      if [[ "$GITHUB_MENU_CHOICE" == "1" ]]; then
        echo ""
        break 2
      fi

      if [[ "$GITHUB_MENU_CHOICE" == "2" ]]; then
        echo ""
        break
      fi

      if [[ "$GITHUB_MENU_CHOICE" == "3" ]]; then
        echo ""
        echo "[INFO] Exiting."
        exit 0
      fi

      echo "[ERROR] Invalid choice. Please enter 1, 2, or 3."
      echo ""
    done
    continue
  fi

  if [[ "$MAIN_CHOICE" == "3" ]]; then
    echo ""

    while true; do
      echo "What would you like to do?"
      echo "  1 = Run Silverbullet"
      echo "  2 = Menu"
      echo "  3 = Quit"
      echo ""
      read -r -p "Enter your choice (1, 2, or 3): " SILVERBULLET_CHOICE
      clear

      if [[ "$SILVERBULLET_CHOICE" == "1" ]]; then
        echo ""
        "$SCRIPT_DIR/run_silverbullet.sh"
        echo ""
        continue
      fi

      if [[ "$SILVERBULLET_CHOICE" == "2" ]]; then
        echo ""
        break
      fi

      if [[ "$SILVERBULLET_CHOICE" == "3" ]]; then
        echo ""
        echo "[INFO] Exiting."
        exit 0
      fi

      echo "[ERROR] Invalid choice. Please enter 1, 2, or 3."
      echo ""
    done
    continue
  fi

  if [[ "$MAIN_CHOICE" == "4" ]]; then
    while true; do
      echo "What would you like to do?"
      echo "  1 = Check Internet"
      echo "  2 = CLI Dependencies"
      echo "  3 = Menu"
      echo "  4 = Quit"
      echo ""
      read -r -p "Enter your choice (1, 2, 3, or 4): " SETTINGS_CHOICE
      clear

      if [[ "$SETTINGS_CHOICE" == "1" ]]; then
        echo ""
        bash "$SCRIPT_DIR/check_internet.sh"
        continue
      fi

      if [[ "$SETTINGS_CHOICE" == "2" ]]; then
        echo ""
        bash "$SCRIPT_DIR/check_dependencies.sh"
        echo ""
        continue
      fi

      if [[ "$SETTINGS_CHOICE" == "3" ]]; then
        echo ""
        break
      fi

      if [[ "$SETTINGS_CHOICE" == "4" ]]; then
        echo ""
        echo "[INFO] Exiting."
        exit 0
      fi

      echo "[ERROR] Invalid choice. Please enter 1, 2, 3, or 4."
      echo ""
    done
    continue
  fi

  if [[ "$MAIN_CHOICE" == "5" ]]; then
    echo ""
    echo "[INFO] Exiting."
    exit 0
  fi

  echo "[ERROR] Invalid choice. Please enter 1, 2, 3, 4, or 5."
done

# ---------------------------------------------------------------------------
# GITHUB BACKUP - USERNAME PROMPT
# ---------------------------------------------------------------------------

echo "# ============================"
echo "# vecnode"
echo "# GitHub Repository Backup"
echo "# ============================"
echo ""

while true; do
  echo ""
  read -r -p "Enter GitHub username: " GITHUB_USERNAME
  if [[ -z "${GITHUB_USERNAME}" ]]; then
    echo "[ERROR] GitHub username cannot be empty."
    continue
  fi
  break
done

echo "[INFO] GitHub username set to: ${GITHUB_USERNAME}"
echo ""

# ---------------------------------------------------------------------------
# GITHUB BACKUP - SOURCE CHOICE
# ---------------------------------------------------------------------------

while true; do
  echo ""
  echo "What would you like to download?"
  echo "  1 = Personal repositories only"
  echo "  2 = Organizations only"
  echo "  3 = Both personal repositories and organizations"
  echo "  4 = Menu"
  echo "  5 = Quit"
  echo ""
  read -r -p "Enter your choice (1, 2, 3, 4, or 5): " SOURCE_CHOICE
  clear

  if [[ "$SOURCE_CHOICE" == "4" ]]; then
    echo ""
    exec "$SCRIPT_DIR/main.sh"
  fi

  if [[ "$SOURCE_CHOICE" == "5" ]]; then
    echo ""
    echo "[INFO] Exiting."
    exit 0
  fi

  if [[ "$SOURCE_CHOICE" != "1" && "$SOURCE_CHOICE" != "2" && "$SOURCE_CHOICE" != "3" ]]; then
    echo "[ERROR] Invalid choice. Please enter 1, 2, 3, 4, or 5."
    continue
  fi

  TIMESTAMP="$(date '+%d-%m-%Y-%H-%M-%S')"
  if [[ "$SOURCE_CHOICE" == "2" ]]; then
    DEFAULT_DOWNLOAD_TARGET="$HOME/Desktop/git-backup-orgs-${TIMESTAMP}"
  else
    DEFAULT_DOWNLOAD_TARGET="$HOME/Desktop/git-backup-${TIMESTAMP}"
  fi

  echo ""
  read -r -p "Where should the repositories be downloaded? (press Enter for default: ${DEFAULT_DOWNLOAD_TARGET}): " DOWNLOAD_TARGET_INPUT

  if [[ -z "$DOWNLOAD_TARGET_INPUT" ]]; then
    DOWNLOAD_TARGET_DIR="$DEFAULT_DOWNLOAD_TARGET"
  else
    DOWNLOAD_TARGET_DIR="$DOWNLOAD_TARGET_INPUT"
  fi

  DOWNLOAD_TARGET_DIR="${DOWNLOAD_TARGET_DIR/#\~/$HOME}"
  echo "[INFO] Download target set to: ${DOWNLOAD_TARGET_DIR}"
  echo ""

  if [[ "$SOURCE_CHOICE" == "1" ]]; then
    echo ""
    echo "[INFO] Downloading personal repositories for \"${GITHUB_USERNAME}\""
    echo ""
    VECNODE_TARGET_DIR="$DOWNLOAD_TARGET_DIR" "$SCRIPT_DIR/download_all_repos.sh" "$GITHUB_USERNAME"
    break
  fi

  if [[ "$SOURCE_CHOICE" == "2" ]]; then
    echo ""
    echo "[INFO] Downloading organization repositories"
    echo ""
    VECNODE_TARGET_DIR="$DOWNLOAD_TARGET_DIR" "$SCRIPT_DIR/download_all_orgs.sh"
    break
  fi

  if [[ "$SOURCE_CHOICE" == "3" ]]; then
    echo ""
    echo "[INFO] Downloading personal repositories for \"${GITHUB_USERNAME}\""
    echo ""
    VECNODE_TARGET_DIR="$DOWNLOAD_TARGET_DIR" "$SCRIPT_DIR/download_all_repos.sh" "$GITHUB_USERNAME"
    echo ""
    echo "[INFO] Downloading organization repositories"
    echo ""
    VECNODE_TARGET_DIR="$DOWNLOAD_TARGET_DIR" "$SCRIPT_DIR/download_all_orgs.sh"
    break
  fi

  echo "[ERROR] Invalid choice. Please enter 1, 2, 3, 4, or 5."
done

# ---------------------------------------------------------------------------
# COMPLETION
# ---------------------------------------------------------------------------

echo ""
echo "# ============================"
echo "# Backup process completed"
echo "# ============================"
