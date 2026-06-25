#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# run_pdfding.sh
# Open the PdfDing PDF manager in Docker, then launch Chrome at its port.
# Persistent data (sqlite db + uploaded PDFs) is kept in the repo's gitignored
# library/.pdfding-data/ so it never reaches GitHub. PDFs are added through the
# PdfDing web UI (it is upload-based, not a watched folder).
#
# Image: mrmn/pdfding:latest   UI: http://localhost:8000
# Requirements (Linux): docker
# Note: PdfDing runs as a non-root user; if the bind-mounted dirs are not
# writable by the container, run: chmod -R 777 library/.pdfding-data
# ---------------------------------------------------------------------------

IMAGE="mrmn/pdfding:latest"
CONTAINER="pdfding"
PORT="8000"
URL="http://localhost:8000"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DATA="$REPO_ROOT/library/.pdfding-data"
DBDIR="$DATA/db"
MEDIADIR="$DATA/media"
SECRET_FILE="$DATA/secret_key.txt"

echo "[INFO] PdfDing (Docker)"
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

mkdir -p "$DBDIR" "$MEDIADIR"

# SECRET_KEY: generate once and persist so sessions survive restarts.
if [[ ! -s "$SECRET_FILE" ]]; then
  echo "[INFO] Generating a persistent SECRET_KEY..."
  if command -v openssl >/dev/null 2>&1; then
    openssl rand -hex 48 > "$SECRET_FILE"
  else
    head -c 48 /dev/urandom | od -An -tx1 | tr -d ' \n' > "$SECRET_FILE"
  fi
  chmod 600 "$SECRET_FILE" 2>/dev/null || true
fi
SECRET_KEY="$(tr -d '\r\n' < "$SECRET_FILE")"
if [[ -z "$SECRET_KEY" ]]; then
  echo "[ERROR] Could not read SECRET_KEY from $SECRET_FILE"; exit 1
fi

echo ""

if docker ps -q --filter "name=^/${CONTAINER}$" | grep -q .; then
  echo "[OK] Container '${CONTAINER}' is already running."
elif docker ps -aq --filter "name=^/${CONTAINER}$" | grep -q .; then
  echo "[INFO] Starting existing container '${CONTAINER}'..."
  docker start "${CONTAINER}" >/dev/null
else
  echo "[INFO] Running image '${IMAGE}'. First run downloads it; this can take a while..."
  docker run -d --name "${CONTAINER}" \
    -p "${PORT}:8000" \
    -e HOST_NAME=localhost,127.0.0.1 \
    -e SECRET_KEY="${SECRET_KEY}" \
    -e CSRF_COOKIE_SECURE=FALSE \
    -e SESSION_COOKIE_SECURE=FALSE \
    -e ACCOUNT_DEFAULT_HTTP_PROTOCOL=http \
    -v "${DBDIR}:/home/nonroot/pdfding/db" \
    -v "${MEDIADIR}:/home/nonroot/pdfding/media" \
    "${IMAGE}" >/dev/null
fi

echo "[INFO] Waiting for PdfDing to become ready at ${URL} ..."
READY=0
for _ in $(seq 1 30); do
  if curl -s -o /dev/null -m 3 "${URL}"; then READY=1; break; fi
  sleep 2
done
if [[ "${READY}" -eq 1 ]]; then echo "[OK] PdfDing is ready."; else echo "[WARNING] PdfDing did not respond yet; opening the browser anyway."; fi

# Prefer Chrome; fall back to a generic opener.
CHROME=""
for c in google-chrome google-chrome-stable chromium chromium-browser; do
  if command -v "$c" >/dev/null 2>&1; then CHROME="$c"; break; fi
done
if [[ -n "$CHROME" ]]; then
  echo "[INFO] Opening Chrome at ${URL}"; "$CHROME" "${URL}" >/dev/null 2>&1 &
elif command -v xdg-open >/dev/null 2>&1; then
  echo "[INFO] Chrome not found; opening default browser at ${URL}"; xdg-open "${URL}" >/dev/null 2>&1 &
else
  echo "[INFO] No browser opener found. Open ${URL} manually."
fi

echo ""
echo "[INFO] Open:  ${URL}"
echo "[INFO] First time: create an account, then upload PDFs from library/pdfs/ via the web UI."
echo "[INFO] Stop with:  vn run ubuntu22-stop-pdfding  (or: docker stop ${CONTAINER})"
echo "[INFO] Logs with:  docker logs -f ${CONTAINER}"
exit 0
