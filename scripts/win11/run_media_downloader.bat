@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM run_media_downloader.bat
REM Build the small media-downloader image (yt-dlp + ffmpeg) and run it, then
REM open Chrome. No host folder is mounted; each download streams straight to
REM the browser and the server-side temp copy is deleted right after.
REM
REM Image: vecnode-media-downloader (built locally)   UI: http://localhost:8095
REM Requirements (Windows): docker
REM ---------------------------------------------------------------------------

set "IMAGE=vecnode-media-downloader"
set "CONTAINER=media-downloader"
set "PORT=8095"
set "URL=http://localhost:8095"

for %%I in ("%~dp0..\..") do set "REPO_ROOT=%%~fI"
set "CTX=%REPO_ROOT%\docker\media-downloader"
set "BUILD_LOG=%TEMP%\vecnode_media_downloader_build.log"

echo [INFO] Media Downloader (Docker)
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

echo [INFO] Building image '%IMAGE%'...
docker build -t %IMAGE% "%CTX%" >"%BUILD_LOG%" 2>&1
if errorlevel 1 (
    echo [ERROR] Image build failed.
    type "%BUILD_LOG%"
    exit /b 1
)
echo [OK] Image built.

echo [INFO] Starting container...
docker rm -f %CONTAINER% >nul 2>nul
docker run -d --name %CONTAINER% -p %PORT%:8095 %IMAGE% >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Failed to start Media Downloader container.
    exit /b 1
)

echo [INFO] Waiting for Media Downloader at %URL% ...
set /a TRIES=0
:wait_loop
set /a TRIES+=1
curl -s -o nul -m 3 "%URL%/health" >nul 2>nul
if not errorlevel 1 goto :ready
if !TRIES! GEQ 20 (
    echo [WARNING] Service did not respond yet; opening the browser anyway.
    goto :open
)
ping -n 2 127.0.0.1 >nul 2>nul
goto :wait_loop

:ready
echo [OK] Media Downloader is ready.

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
echo [INFO] Paste a video URL, pick MP3 / WAV / MP4 - the browser downloads the result.
echo [INFO] Stop with:  vn run win11-stop-media-downloader  ^(or: docker stop %CONTAINER%^)
echo [INFO] Logs with:  docker logs -f %CONTAINER%
endlocal
exit /b 0
