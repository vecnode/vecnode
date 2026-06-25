@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM run_papra.bat
REM Open the Papra document app in Docker, then launch Chrome at its port.
REM Mounts the repo's gitignored library/ as Papra's ingestion folder and keeps
REM Papra's own data in library/.papra-data/ (never pushed to GitHub).
REM
REM Image: ghcr.io/papra-hq/papra:latest   UI: http://localhost:1221
REM Requirements (Windows): docker
REM ---------------------------------------------------------------------------

set "IMAGE=ghcr.io/papra-hq/papra:latest"
set "CONTAINER=papra"
set "PORT=1221"
set "URL=http://localhost:1221"

for %%I in ("%~dp0..\..") do set "REPO_ROOT=%%~fI"
set "LIB=%REPO_ROOT%\library"
set "DATA=%LIB%\.papra-data"
set "SECRET_FILE=%DATA%\auth_secret.txt"
set "IGNORED=**/.DS_Store,**/.env,**/desktop.ini,**/Thumbs.db,**/.git/**,**/.idea/**,**/.vscode/**,**/node_modules/**,**/@eaDir/**,**/*@SynoResource,**/*@SynoEAStream,**/.papra-data/**"

echo [INFO] Papra (Docker)
echo.

REM --- Docker availability ---
where docker >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Docker is not available or not in PATH.
    echo Install Docker Desktop: https://docs.docker.com/engine/install/
    exit /b 1
)
docker info >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Docker daemon is not running. Start Docker and try again.
    exit /b 1
)
echo [OK] Docker daemon is running.

REM --- Folders (library/ is the ingestion inbox; .papra-data/ is Papra's store) ---
if not exist "%LIB%" mkdir "%LIB%" >nul 2>nul
if not exist "%DATA%" mkdir "%DATA%" >nul 2>nul

REM --- AUTH_SECRET: generate once and persist so logins/data survive restarts ---
if not exist "%SECRET_FILE%" goto :gen_secret
goto :read_secret

:gen_secret
echo [INFO] Generating a persistent AUTH_SECRET...
powershell -NoProfile -Command "[IO.File]::WriteAllText('%SECRET_FILE%', [guid]::NewGuid().ToString('N')+[guid]::NewGuid().ToString('N')+[guid]::NewGuid().ToString('N'))"

:read_secret
set "AUTH_SECRET="
set /p AUTH_SECRET=<"%SECRET_FILE%"
if not defined AUTH_SECRET (
    echo [ERROR] Could not read AUTH_SECRET from %SECRET_FILE%
    exit /b 1
)

echo.

REM --- Already running? (capture id to avoid depending on find.exe in PATH) ---
set "RUNNING="
for /f "delims=" %%i in ('docker ps -q --filter "name=^/%CONTAINER%$" 2^>nul') do set "RUNNING=%%i"
if defined RUNNING (
    echo [OK] Container '%CONTAINER%' is already running.
    goto :wait
)

REM --- Stopped container exists? start it. Otherwise run a new one. ---
set "EXISTS="
for /f "delims=" %%i in ('docker ps -aq --filter "name=^/%CONTAINER%$" 2^>nul') do set "EXISTS=%%i"
if defined EXISTS (
    echo [INFO] Starting existing container '%CONTAINER%'...
    docker start %CONTAINER% >nul 2>nul
    if errorlevel 1 (
        echo [ERROR] Failed to start existing container '%CONTAINER%'.
        exit /b 1
    )
    goto :wait
)

echo [INFO] Running image '%IMAGE%'. First run downloads it; this can take a while...
docker run -d --name %CONTAINER% -p %PORT%:1221 -e APP_BASE_URL=%URL% -e AUTH_SECRET=%AUTH_SECRET% -e INGESTION_FOLDER_IS_ENABLED=true -e INGESTION_FOLDER_ROOT_PATH=/app/ingestion -e INGESTION_FOLDER_WATCHER_USE_POLLING=true -e INGESTION_FOLDER_POST_PROCESSING_STRATEGY=move -e INGESTION_FOLDER_POST_PROCESSING_MOVE_FOLDER_PATH=./_ingested -e "INGESTION_FOLDER_IGNORED_PATTERNS=%IGNORED%" -v "%LIB%:/app/ingestion" -v "%DATA%:/app/app-data" %IMAGE% >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Failed to start Papra container.
    exit /b 1
)

:wait
echo [INFO] Waiting for Papra to become ready at %URL% ...
set /a TRIES=0
:wait_loop
set /a TRIES+=1
curl -s -o nul -m 3 "%URL%" >nul 2>nul
if not errorlevel 1 goto :ready
if !TRIES! GEQ 30 (
    echo [WARNING] Papra did not respond yet; opening the browser anyway.
    goto :open
)
ping -n 3 127.0.0.1 >nul 2>nul
goto :wait_loop

:ready
echo [OK] Papra is ready.

:open
REM Prefer Chrome; fall back to the default browser if Chrome is not installed.
set "CHROME="
if exist "%ProgramFiles%\Google\Chrome\Application\chrome.exe" set "CHROME=%ProgramFiles%\Google\Chrome\Application\chrome.exe"
if not defined CHROME if exist "%ProgramFiles(x86)%\Google\Chrome\Application\chrome.exe" set "CHROME=%ProgramFiles(x86)%\Google\Chrome\Application\chrome.exe"
if not defined CHROME if exist "%LOCALAPPDATA%\Google\Chrome\Application\chrome.exe" set "CHROME=%LOCALAPPDATA%\Google\Chrome\Application\chrome.exe"

if defined CHROME (
    echo [INFO] Opening Chrome at %URL%
    start "" "%CHROME%" "%URL%"
) else (
    echo [INFO] Chrome not found; opening default browser at %URL%
    start "" "%URL%"
)

echo.
echo [INFO] Open:  %URL%
echo [INFO] First time: sign up, create an organization, then put PDFs in library\^<org-slug^>\
echo [INFO] Imported files are moved to library\^<org-slug^>\_ingested\
echo [INFO] Stop with:  vn run win11-stop-papra  ^(or: docker stop %CONTAINER%^)
echo [INFO] Logs with:  docker logs -f %CONTAINER%
endlocal
exit /b 0
