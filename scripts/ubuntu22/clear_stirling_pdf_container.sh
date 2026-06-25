#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# clear_stirling_pdf_container.sh
# Remove the Stirling-PDF container (force). The image is left in place, so a
# later open is fast (no re-download). Use clear_stirling_pdf_image.sh to also
# remove the image.
# ---------------------------------------------------------------------------

CONTAINER="stirling-pdf"

if ! command -v docker >/dev/null 2>&1; then
  echo "[ERROR] Docker is not available or not in PATH."
  exit 1
fi

if ! docker ps -a --filter "name=^/${CONTAINER}$" --format '{{.Names}}' | grep -qi "${CONTAINER}"; then
  echo "[INFO] No '${CONTAINER}' container exists. Nothing to clear."
  exit 0
fi

echo "[INFO] Removing container '${CONTAINER}'..."
docker rm -f "${CONTAINER}" >/dev/null
echo "[OK] Removed container '${CONTAINER}'."
exit 0
