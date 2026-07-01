@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM run_stirling_pdf.bat
REM Open the Stirling-PDF web app in Docker, then launch Chrome at its port.
REM Pulls the image on first run, reuses/starts an existing container otherwise.
REM
REM Image: stirlingtools/stirling-pdf:latest   UI: http://localhost:8080
REM Requirements (Windows): docker
REM ---------------------------------------------------------------------------

set "IMAGE=stirlingtools/stirling-pdf:latest"
set "CONTAINER=stirling-pdf"
set "PORT=8080"
set "URL=http://localhost:8080"

echo [INFO] Stirling-PDF (Docker)
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
) else (
    echo [INFO] Running image '%IMAGE%'. First run downloads it; this can take a while...
    docker run -d --name %CONTAINER% -p 127.0.0.1:%PORT%:8080 %IMAGE% >nul 2>nul
    if errorlevel 1 (
        echo [ERROR] Failed to start Stirling-PDF container.
        exit /b 1
    )
)

:wait
echo [INFO] Waiting for Stirling-PDF to become ready at %URL% ...
set /a TRIES=0
:wait_loop
set /a TRIES+=1
curl -s -o nul -m 3 "%URL%" >nul 2>nul
if not errorlevel 1 goto :ready
if !TRIES! GEQ 30 (
    echo [WARNING] Stirling-PDF did not respond yet; opening the browser anyway.
    goto :open
)
ping -n 3 127.0.0.1 >nul 2>nul
goto :wait_loop

:ready
echo [OK] Stirling-PDF is ready.

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
echo [INFO] Stop with:  vn run win11-stop-stirling-pdf  ^(or: docker stop %CONTAINER%^)
echo [INFO] Logs with:  docker logs -f %CONTAINER%
endlocal
exit /b 0
