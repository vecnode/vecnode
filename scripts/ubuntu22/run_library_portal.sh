#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# run_library_portal.sh
# Build the super-light library-portal image and run it with the repo's
# library/ folder bind-mounted READ-ONLY, then open Chrome. Nothing is copied
# into the image or written to disk; the portal just serves what is in library/.
#
# Image: vecnode-library-portal (built locally)   UI: http://localhost:8090
# Requirements (Linux): docker
# ---------------------------------------------------------------------------

IMAGE="vecnode-library-portal"
CONTAINER="library-portal"
PORT="8090"
URL="http://localhost:8090"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CTX="$REPO_ROOT/docker/library-portal"

echo "[INFO] Library Portal (Docker)"
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

echo "[INFO] Building image '${IMAGE}' - app only, no PDFs are copied in..."
docker build -t "${IMAGE}" "${CTX}"
echo "[OK] Image built."

echo "[INFO] Starting container with library/ mounted (non-root, caps dropped)..."
docker rm -f "${CONTAINER}" >/dev/null 2>&1 || true
docker run -d --name "${CONTAINER}" \
  --user "$(id -u):$(id -g)" \
  --cap-drop ALL --security-opt no-new-privileges --pids-limit 512 \
  -p "127.0.0.1:${PORT}:8090" \
  -v "${REPO_ROOT}/library:/library" \
  "${IMAGE}" >/dev/null

echo "[INFO] Waiting for Library Portal at ${URL} ..."
READY=0
for _ in $(seq 1 20); do
  if curl -s -o /dev/null -m 3 "${URL}/health"; then READY=1; break; fi
  sleep 1.5
done
if [[ "${READY}" -eq 1 ]]; then echo "[OK] Library Portal is ready."; else echo "[WARNING] Portal did not respond yet; opening the browser anyway."; fi

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
echo "[INFO] No PDFs are copied into the image. Tags/edits and thumbnails are stored in library/.portal/."
echo "[INFO] Stop with:  vn run ubuntu22-stop-library-portal  (or: docker stop ${CONTAINER})"
echo "[INFO] Logs with:  docker logs -f ${CONTAINER}"
exit 0
