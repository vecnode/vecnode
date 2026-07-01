@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM run_library_portal.bat
REM Build the super-light library-portal image and run it with the repo's
REM library/ folder bind-mounted READ-ONLY, then open Chrome. Nothing is copied
REM into the image or written to disk; the portal just serves what is in library/.
REM
REM Image: vecnode-library-portal (built locally)   UI: http://localhost:8090
REM Requirements (Windows): docker
REM ---------------------------------------------------------------------------

set "IMAGE=vecnode-library-portal"
set "CONTAINER=library-portal"
set "PORT=8090"
set "URL=http://localhost:8090"

for %%I in ("%~dp0..\..") do set "REPO_ROOT=%%~fI"
set "CTX=%REPO_ROOT%\docker\library-portal"
set "BUILD_LOG=%TEMP%\vecnode_library_portal_build.log"

echo [INFO] Library Portal (Docker)
echo.

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

echo [INFO] Building image '%IMAGE%' - app only, no PDFs are copied in...
docker build -t %IMAGE% "%CTX%" >"%BUILD_LOG%" 2>&1
if errorlevel 1 (
    echo [ERROR] Image build failed.
    type "%BUILD_LOG%"
    exit /b 1
)
echo [OK] Image built.

echo [INFO] Starting container with library/ mounted (non-root, caps dropped)...
docker rm -f %CONTAINER% >nul 2>nul
docker run -d --name %CONTAINER% --cap-drop ALL --security-opt no-new-privileges --pids-limit 512 -p 127.0.0.1:%PORT%:8090 -v "%REPO_ROOT%\library:/library" %IMAGE% >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Failed to start Library Portal container.
    exit /b 1
)

echo [INFO] Waiting for Library Portal at %URL% ...
set /a TRIES=0
:wait_loop
set /a TRIES+=1
curl -s -o nul -m 3 "%URL%/health" >nul 2>nul
if not errorlevel 1 goto :ready
if !TRIES! GEQ 20 (
    echo [WARNING] Portal did not respond yet; opening the browser anyway.
    goto :open
)
ping -n 2 127.0.0.1 >nul 2>nul
goto :wait_loop

:ready
echo [OK] Library Portal is ready.

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
echo [INFO] No PDFs are copied into the image. Tags/edits and thumbnails are stored in library\.portal\.
echo [INFO] Stop with:  vn run win11-stop-library-portal  ^(or: docker stop %CONTAINER%^)
echo [INFO] Logs with:  docker logs -f %CONTAINER%
endlocal
exit /b 0
