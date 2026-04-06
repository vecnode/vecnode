#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# download_all_orgs.sh
# Clone all public repositories from a fixed list of GitHub organizations.
#
# Usage:
#   ./download_all_orgs.sh
#
# Downloads into: ~/Desktop/git-backup-orgs-DD-MM-YYYY-HH-MM-SS/
# ---------------------------------------------------------------------------

# ── Configuration ────────────────────────────────────────────────────────────
TIMESTAMP="$(date '+%d-%m-%Y-%H-%M-%S')"
TARGET_DIR="$HOME/Desktop/git-backup-orgs-${TIMESTAMP}"
PER_PAGE=100   # max allowed by GitHub API
ORG_LINKS=(
	"https://github.com/sttera-studio"
	"https://github.com/atomic-media-studio"
	"https://github.com/osd-network"
	"https://github.com/arpsci"
)
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

fetch_org_repos() {
	local org_name="$1"
	local page=1
	local repos=()

	while true; do
		local url="https://api.github.com/orgs/${org_name}/repos?per_page=${PER_PAGE}&page=${page}&type=public"
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

	if (( ${#repos[@]} == 0 )); then
		return
	fi

	printf '%s\n' "${repos[@]}"
}

mkdir -p "$TARGET_DIR"
echo "[INFO] Syncing hardcoded organizations into '$TARGET_DIR'"

CLONED=0
PULLED=0
FAILED=0
ORGS_COUNT=0
ORG_REPOS_TOTAL=0

for org_link in "${ORG_LINKS[@]}"; do
	org_name="${org_link##*/}"
	(( ORGS_COUNT++ )) || true

	org_dir="$TARGET_DIR/$org_name"
	mkdir -p "$org_dir"

	echo ""
	echo "[ORG $ORGS_COUNT/${#ORG_LINKS[@]}] $org_name"

	org_repos=$(fetch_org_repos "$org_name")
	if [[ -z "$org_repos" ]]; then
		echo "         no public repositories"
		continue
	fi

	org_total=$(echo "$org_repos" | wc -l)
	(( ORG_REPOS_TOTAL += org_total )) || true
	org_idx=0

	while IFS= read -r repo_url; do
		(( org_idx++ )) || true
		repo_name=$(basename "$repo_url" .git)
		repo_path="$org_dir/$repo_name"

		echo "         [$org_idx/$org_total] $repo_name"

		if [[ -d "$repo_path/.git" ]]; then
			if git -C "$repo_path" pull --ff-only --quiet 2>&1; then
				echo "                  ✔ pulled"
				(( PULLED++ )) || true
			else
				echo "                  ✘ pull failed (skipped)"
				(( FAILED++ )) || true
			fi
		else
			if git clone --quiet "$repo_url" "$repo_path" 2>&1; then
				echo "                  ✔ cloned"
				(( CLONED++ )) || true
			else
				echo "                  ✘ clone failed (skipped)"
				(( FAILED++ )) || true
			fi
		fi
	done <<< "$org_repos"
done

echo ""
echo "────────────────────────────────────────"
echo " Done. cloned=$CLONED pulled=$PULLED failed=$FAILED"
echo "       orgs=$ORGS_COUNT org_repos=$ORG_REPOS_TOTAL"
echo "────────────────────────────────────────"

