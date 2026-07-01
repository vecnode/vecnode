#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# run_stirling_pdf.sh
# Open the Stirling-PDF web app in Docker, then launch Chrome at its port.
# Pulls the image on first run, reuses/starts an existing container otherwise.
#
# Image: stirlingtools/stirling-pdf:latest   UI: http://localhost:8080
# Requirements (Linux): docker
# ---------------------------------------------------------------------------

IMAGE="stirlingtools/stirling-pdf:latest"
CONTAINER="stirling-pdf"
PORT="8080"
URL="http://localhost:8080"

echo "[INFO] Stirling-PDF (Docker)"
echo ""

if ! command -v docker >/dev/null 2>&1; then
  echo "[ERROR] Docker is not available or not in PATH."
  echo "Install Docker Engine: https://docs.docker.com/engine/install/"
  exit 1
fi
if ! docker info >/dev/null 2>&1; then
  echo "[ERROR] Docker daemon is not running. Start Docker and try again."
  exit 1
fi
echo "[OK] Docker daemon is running."
echo ""

if docker ps --filter "name=^/${CONTAINER}$" --format '{{.Names}}' | grep -qi "${CONTAINER}"; then
  echo "[OK] Container '${CONTAINER}' is already running."
elif docker ps -a --filter "name=^/${CONTAINER}$" --format '{{.Names}}' | grep -qi "${CONTAINER}"; then
  echo "[INFO] Starting existing container '${CONTAINER}'..."
  docker start "${CONTAINER}" >/dev/null
else
  echo "[INFO] Running image '${IMAGE}' (first run downloads it, this can take a while)..."
  docker run -d --name "${CONTAINER}" -p "127.0.0.1:${PORT}:8080" "${IMAGE}" >/dev/null
fi

echo "[INFO] Waiting for Stirling-PDF to become ready at ${URL} ..."
READY=0
for _ in $(seq 1 30); do
  if curl -s -o /dev/null -m 3 "${URL}"; then
    READY=1
    break
  fi
  sleep 2
done

if [[ "${READY}" -eq 1 ]]; then
  echo "[OK] Stirling-PDF is ready."
else
  echo "[WARNING] Stirling-PDF did not respond yet; opening the browser anyway."
fi

# Prefer Chrome; fall back to a generic opener.
CHROME=""
for candidate in google-chrome google-chrome-stable chromium chromium-browser; do
  if command -v "${candidate}" >/dev/null 2>&1; then
    CHROME="${candidate}"
    break
  fi
done

if [[ -n "${CHROME}" ]]; then
  echo "[INFO] Opening Chrome at ${URL}"
  "${CHROME}" "${URL}" >/dev/null 2>&1 &
elif command -v xdg-open >/dev/null 2>&1; then
  echo "[INFO] Chrome not found; opening default browser at ${URL}"
  xdg-open "${URL}" >/dev/null 2>&1 &
else
  echo "[INFO] No browser opener found. Open ${URL} manually."
fi

echo ""
echo "[INFO] Open:  ${URL}"
echo "[INFO] Stop with:  vn run ubuntu22-stop-stirling-pdf  (or: docker stop ${CONTAINER})"
echo "[INFO] Logs with:  docker logs -f ${CONTAINER}"
exit 0
