@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM run_pdfding.bat
REM Open the PdfDing PDF manager in Docker, then launch Chrome at its port.
REM Persistent data (sqlite db + uploaded PDFs) is kept in the repo's gitignored
REM library/.pdfding-data/ so it never reaches GitHub. PDFs are added through the
REM PdfDing web UI (it is upload-based, not a watched folder).
REM
REM Image: mrmn/pdfding:latest   UI: http://localhost:8000
REM Requirements (Windows): docker
REM ---------------------------------------------------------------------------

set "IMAGE=mrmn/pdfding:latest"
set "CONTAINER=pdfding"
set "PORT=8000"
set "URL=http://localhost:8000"

for %%I in ("%~dp0..\..") do set "REPO_ROOT=%%~fI"
set "DATA=%REPO_ROOT%\library\.pdfding-data"
set "DBDIR=%DATA%\db"
set "MEDIADIR=%DATA%\media"
set "SECRET_FILE=%DATA%\secret_key.txt"

echo [INFO] PdfDing (Docker)
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

REM --- Persistent data folders (gitignored, under library/) ---
if not exist "%DATA%" mkdir "%DATA%" >nul 2>nul
if not exist "%DBDIR%" mkdir "%DBDIR%" >nul 2>nul
if not exist "%MEDIADIR%" mkdir "%MEDIADIR%" >nul 2>nul

REM --- SECRET_KEY: generate once and persist so sessions survive restarts ---
if not exist "%SECRET_FILE%" goto :gen_secret
goto :read_secret

:gen_secret
echo [INFO] Generating a persistent SECRET_KEY...
powershell -NoProfile -Command "[IO.File]::WriteAllText('%SECRET_FILE%', [guid]::NewGuid().ToString('N')+[guid]::NewGuid().ToString('N')+[guid]::NewGuid().ToString('N'))"

:read_secret
set "SECRET_KEY="
set /p SECRET_KEY=<"%SECRET_FILE%"
if not defined SECRET_KEY (
    echo [ERROR] Could not read SECRET_KEY from %SECRET_FILE%
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
docker run -d --name %CONTAINER% -p %PORT%:8000 -e "HOST_NAME=localhost,127.0.0.1" -e SECRET_KEY=%SECRET_KEY% -e CSRF_COOKIE_SECURE=FALSE -e SESSION_COOKIE_SECURE=FALSE -e ACCOUNT_DEFAULT_HTTP_PROTOCOL=http -v "%DBDIR%:/home/nonroot/pdfding/db" -v "%MEDIADIR%:/home/nonroot/pdfding/media" %IMAGE% >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Failed to start PdfDing container.
    exit /b 1
)

:wait
echo [INFO] Waiting for PdfDing to become ready at %URL% ...
set /a TRIES=0
:wait_loop
set /a TRIES+=1
curl -s -o nul -m 3 "%URL%" >nul 2>nul
if not errorlevel 1 goto :ready
if !TRIES! GEQ 30 (
    echo [WARNING] PdfDing did not respond yet; opening the browser anyway.
    goto :open
)
ping -n 3 127.0.0.1 >nul 2>nul
goto :wait_loop

:ready
echo [OK] PdfDing is ready.

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
echo [INFO] First time: create an account, then upload PDFs from library\pdfs\ via the web UI.
echo [INFO] Stop with:  vn run win11-stop-pdfding  ^(or: docker stop %CONTAINER%^)
echo [INFO] Logs with:  docker logs -f %CONTAINER%
endlocal
exit /b 0
