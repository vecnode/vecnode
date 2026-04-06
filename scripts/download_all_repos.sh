#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# download_all_repos.sh
# Clone all public repositories from a GitHub account.
#
# Usage:
#   ./download_all_repos.sh
#
# Downloads into: ~/Desktop/git-backup-DD-MM-YYYY-HH-MM-SS/
#
# Requirements (Linux):
#   - git
#   - curl
#   - jq
# ---------------------------------------------------------------------------

# ── Configuration ────────────────────────────────────────────────────────────
GITHUB_USER="vecnode"
TIMESTAMP="$(date '+%d-%m-%Y-%H-%M-%S')"
TARGET_DIR="$HOME/Desktop/git-backup-${TIMESTAMP}"
PER_PAGE=100   # max allowed by GitHub API
# ─────────────────────────────────────────────────────────────────────────────

# ── OS check ─────────────────────────────────────────────────────────────────
OS="$(uname -s)"
if [[ "$OS" != "Linux" ]]; then
  echo "[ERROR] This script is designed for Linux (detected: $OS)."
  exit 1
fi
# ─────────────────────────────────────────────────────────────────────────────

# ── Dependency check ─────────────────────────────────────────────────────────
for cmd in git curl jq; do
  if ! command -v "$cmd" &>/dev/null; then
    echo "[ERROR] Required command not found: $cmd"
    echo "        Install it with:  sudo apt install $cmd"
    exit 1
  fi
done
# ─────────────────────────────────────────────────────────────────────────────

mkdir -p "$TARGET_DIR"
echo "[INFO] Syncing repos for '$GITHUB_USER' into '$TARGET_DIR'"
echo ""

# ── Fetch full repo list (handles pagination) ─────────────────────────────────
fetch_repos() {
  local page=1
  local repos=()

  while true; do
    local url="https://api.github.com/users/${GITHUB_USER}/repos?per_page=${PER_PAGE}&page=${page}&type=owner"
    local response

    response=$(curl -fsSL -H "Accept: application/vnd.github+json" "$url")

    local batch
    batch=$(echo "$response" | jq -r '.[].clone_url')

    [[ -z "$batch" ]] && break

    repos+=($batch)

    local count
    count=$(echo "$response" | jq 'length')
    (( count < PER_PAGE )) && break

    (( page++ ))
  done

  printf '%s\n' "${repos[@]}"
}
# ─────────────────────────────────────────────────────────────────────────────

REPOS=$(fetch_repos)

CLONED=0
PULLED=0
FAILED=0

if [[ -z "$REPOS" ]]; then
  echo "[INFO] No personal repositories found for '$GITHUB_USER'."
else
  TOTAL=$(echo "$REPOS" | wc -l)
  IDX=0

  while IFS= read -r repo_url; do
    (( IDX++ )) || true
    # derive folder name from URL (strip .git suffix)
    repo_name=$(basename "$repo_url" .git)
    repo_path="$TARGET_DIR/$repo_name"

    echo "[$IDX/$TOTAL] $repo_name"

    if [[ -d "$repo_path/.git" ]]; then
      # already cloned - pull latest changes
      if git -C "$repo_path" pull --ff-only --quiet 2>&1; then
        echo "         ✔ pulled"
        (( PULLED++ )) || true
      else
        echo "         ✘ pull failed (skipped)"
        (( FAILED++ )) || true
      fi
    else
      # fresh clone
      if git clone --quiet "$repo_url" "$repo_path" 2>&1; then
        echo "         ✔ cloned"
        (( CLONED++ )) || true
      else
        echo "         ✘ clone failed (skipped)"
        (( FAILED++ )) || true
      fi
    fi
  done <<< "$REPOS"
fi

echo ""
echo "────────────────────────────────────────"
echo " Done. cloned=$CLONED pulled=$PULLED failed=$FAILED"
echo "────────────────────────────────────────"
