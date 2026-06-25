#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# stop_pdfding.sh
# Stop the PdfDing container. The container is kept (not removed), so it can be
# reopened quickly with run_pdfding.sh. Your library/.pdfding-data/ (db +
# uploaded PDFs) is untouched.
# ---------------------------------------------------------------------------

CONTAINER="pdfding"

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
