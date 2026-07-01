@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM run_media_downloader.bat
REM Build the small media-downloader image (yt-dlp + ffmpeg) and run it, then
REM open Chrome. Downloaded media is saved to the host Desktop (bind-mounted at
REM /output). The container runs non-root with all capabilities dropped and
REM no-new-privileges, since it fetches from arbitrary web links.
REM
REM Image: vecnode-media-downloader (built locally)   UI: http://localhost:8095
REM Requirements (Windows): docker
REM ---------------------------------------------------------------------------

set "IMAGE=vecnode-media-downloader"
set "CONTAINER=media-downloader"
set "PORT=8095"
set "URL=http://localhost:8095"
set "HOST_DESKTOP=%USERPROFILE%\Desktop"

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

echo [INFO] Starting container (non-root, caps dropped); saving to %HOST_DESKTOP% ...
docker rm -f %CONTAINER% >nul 2>nul
docker run -d --name %CONTAINER% --cap-drop ALL --security-opt no-new-privileges --pids-limit 512 -p 127.0.0.1:%PORT%:8095 -e OUTPUT_LABEL=Desktop -v "%HOST_DESKTOP%:/output" %IMAGE% >nul 2>nul
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
echo [INFO] Paste a video URL, pick MP3 / WAV / MP4 - the file is saved to your Desktop.
echo [INFO] Save folder: %HOST_DESKTOP%
echo [INFO] Stop with:  vn run win11-stop-media-downloader  ^(or: docker stop %CONTAINER%^)
echo [INFO] Logs with:  docker logs -f %CONTAINER%
endlocal
exit /b 0
