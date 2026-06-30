#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# run_media_downloader.sh
# Build the small media-downloader image (yt-dlp + ffmpeg) and run it, then
# open Chrome. No host folder is mounted; each download streams straight to
# the browser and the server-side temp copy is deleted right after.
#
# Image: vecnode-media-downloader (built locally)   UI: http://localhost:8095
# Requirements (Linux): docker
# ---------------------------------------------------------------------------

IMAGE="vecnode-media-downloader"
CONTAINER="media-downloader"
PORT="8095"
URL="http://localhost:8095"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CTX="$REPO_ROOT/docker/media-downloader"

echo "[INFO] Media Downloader (Docker)"
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

echo "[INFO] Building image '${IMAGE}'..."
docker build -t "${IMAGE}" "${CTX}"
echo "[OK] Image built."

echo "[INFO] Starting container..."
docker rm -f "${CONTAINER}" >/dev/null 2>&1 || true
docker run -d --name "${CONTAINER}" -p "${PORT}:8095" "${IMAGE}" >/dev/null

echo "[INFO] Waiting for Media Downloader at ${URL} ..."
READY=0
for _ in $(seq 1 20); do
  if curl -s -o /dev/null -m 3 "${URL}/health"; then READY=1; break; fi
  sleep 1.5
done
if [[ "${READY}" -eq 1 ]]; then echo "[OK] Media Downloader is ready."; else echo "[WARNING] Service did not respond yet; opening the browser anyway."; fi

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
echo "[INFO] Paste a video URL, pick MP3 / WAV / MP4 - the browser downloads the result."
echo "[INFO] Stop with:  vn run ubuntu22-stop-media-downloader  (or: docker stop ${CONTAINER})"
echo "[INFO] Logs with:  docker logs -f ${CONTAINER}"
exit 0
