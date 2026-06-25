#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# stop_papra.sh
# Stop the Papra container. The container is kept (not removed), so it can be
# reopened quickly with run_papra.sh. Your library/ and library/.papra-data/
# are untouched.
# ---------------------------------------------------------------------------

CONTAINER="papra"

if ! command -v docker >/dev/null 2>&1; then
  echo "[ERROR] Docker is not available or not in PATH."
  exit 1
fi

if ! docker ps -aq --filter "name=^/${CONTAINER}$" | grep -q .; then
  echo "[INFO] No '${CONTAINER}' container exists. Nothing to stop."
  exit 0
fi

echo "[INFO] Stopping '${CONTAINER}'..."
docker stop "${CONTAINER}" >/dev/null
echo "[OK] Stopped '${CONTAINER}'. It is kept and can be reopened."
exit 0
