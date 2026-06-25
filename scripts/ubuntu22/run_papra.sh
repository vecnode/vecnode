#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# run_papra.sh
# Open the Papra document app in Docker, then launch Chrome at its port.
# Mounts the repo's gitignored library/ as Papra's ingestion folder and keeps
# Papra's own data in library/.papra-data/ (never pushed to GitHub).
#
# Image: ghcr.io/papra-hq/papra:latest   UI: http://localhost:1221
# Requirements (Linux): docker
# ---------------------------------------------------------------------------

IMAGE="ghcr.io/papra-hq/papra:latest"
CONTAINER="papra"
PORT="1221"
URL="http://localhost:1221"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LIB="$REPO_ROOT/library"
DATA="$LIB/.papra-data"
SECRET_FILE="$DATA/auth_secret.txt"
IGNORED="**/.DS_Store,**/.env,**/desktop.ini,**/Thumbs.db,**/.git/**,**/.idea/**,**/.vscode/**,**/node_modules/**,**/@eaDir/**,**/*@SynoResource,**/*@SynoEAStream,**/.papra-data/**"

echo "[INFO] Papra (Docker)"
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

mkdir -p "$LIB" "$DATA"

# AUTH_SECRET: generate once and persist so logins/data survive restarts.
if [[ ! -s "$SECRET_FILE" ]]; then
  echo "[INFO] Generating a persistent AUTH_SECRET..."
  if command -v openssl >/dev/null 2>&1; then
    openssl rand -hex 48 > "$SECRET_FILE"
  else
    head -c 48 /dev/urandom | od -An -tx1 | tr -d ' \n' > "$SECRET_FILE"
  fi
  chmod 600 "$SECRET_FILE" 2>/dev/null || true
fi
AUTH_SECRET="$(tr -d '\r\n' < "$SECRET_FILE")"
if [[ -z "$AUTH_SECRET" ]]; then
  echo "[ERROR] Could not read AUTH_SECRET from $SECRET_FILE"
  exit 1
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
    --user "$(id -u):$(id -g)" \
    -p "${PORT}:1221" \
    -e APP_BASE_URL="${URL}" \
    -e AUTH_SECRET="${AUTH_SECRET}" \
    -e INGESTION_FOLDER_IS_ENABLED=true \
    -e INGESTION_FOLDER_ROOT_PATH=/app/ingestion \
    -e INGESTION_FOLDER_WATCHER_USE_POLLING=true \
    -e INGESTION_FOLDER_POST_PROCESSING_STRATEGY=move \
    -e INGESTION_FOLDER_POST_PROCESSING_MOVE_FOLDER_PATH=./_ingested \
    -e "INGESTION_FOLDER_IGNORED_PATTERNS=${IGNORED}" \
    -v "${LIB}:/app/ingestion" \
    -v "${DATA}:/app/app-data" \
    "${IMAGE}" >/dev/null
fi

echo "[INFO] Waiting for Papra to become ready at ${URL} ..."
READY=0
for _ in $(seq 1 30); do
  if curl -s -o /dev/null -m 3 "${URL}"; then
    READY=1
    break
  fi
  sleep 2
done

if [[ "${READY}" -eq 1 ]]; then
  echo "[OK] Papra is ready."
else
  echo "[WARNING] Papra did not respond yet; opening the browser anyway."
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
echo "[INFO] First time: sign up, create an organization, then put PDFs in library/<org-slug>/"
echo "[INFO] Imported files are moved to library/<org-slug>/_ingested/"
echo "[INFO] Stop with:  vn run ubuntu22-stop-papra  (or: docker stop ${CONTAINER})"
echo "[INFO] Logs with:  docker logs -f ${CONTAINER}"
exit 0
