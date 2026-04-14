#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# main.sh
# Entry point for vecnode - GitHub repository backup tool
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
echo "# GitHub Repository Backup"
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
  echo "  1 = Backup GitHub"
  echo "  2 = Silverbullet"
  echo "  3 = Quit"
  echo ""
  read -r -p "Enter your choice (1, 2, or 3): " MAIN_CHOICE

  if [[ "$MAIN_CHOICE" == "1" ]]; then
    echo ""
    break
  fi

  if [[ "$MAIN_CHOICE" == "2" ]]; then
    echo ""
    "$SCRIPT_DIR/run_silverbullet.sh"
    exit 0
  fi

  if [[ "$MAIN_CHOICE" == "3" ]]; then
    echo ""
    echo "[INFO] Exiting."
    exit 0
  fi

  echo "[ERROR] Invalid choice. Please enter 1, 2, or 3."
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
  echo ""
  read -r -p "Enter your choice (1, 2, or 3): " SOURCE_CHOICE

  if [[ "$SOURCE_CHOICE" == "1" ]]; then
    echo ""
    echo "[INFO] Downloading personal repositories for \"${GITHUB_USERNAME}\""
    echo ""
    "$SCRIPT_DIR/download_all_repos.sh" "$GITHUB_USERNAME"
    break
  fi

  if [[ "$SOURCE_CHOICE" == "2" ]]; then
    echo ""
    echo "[INFO] Downloading organization repositories"
    echo ""
    "$SCRIPT_DIR/download_all_orgs.sh"
    break
  fi

  if [[ "$SOURCE_CHOICE" == "3" ]]; then
    echo ""
    echo "[INFO] Downloading personal repositories for \"${GITHUB_USERNAME}\""
    echo ""
    "$SCRIPT_DIR/download_all_repos.sh" "$GITHUB_USERNAME"
    echo ""
    echo "[INFO] Downloading organization repositories"
    echo ""
    "$SCRIPT_DIR/download_all_orgs.sh"
    break
  fi

  echo "[ERROR] Invalid choice. Please enter 1, 2, or 3."
done

# ---------------------------------------------------------------------------
# COMPLETION
# ---------------------------------------------------------------------------

echo ""
echo "# ============================"
echo "# Backup process completed"
echo "# ============================"
