@echo off
setlocal EnableExtensions

REM ---------------------------------------------------------------------------
REM stop_media_downloader.bat
REM Stop the media-downloader container. It holds no state, so this just stops
REM it; reopen rebuilds and runs it fresh.
REM ---------------------------------------------------------------------------

set "CONTAINER=media-downloader"

where docker >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Docker is not available or not in PATH.
    exit /b 1
)

set "EXISTS="
for /f "delims=" %%i in ('docker ps -aq --filter "name=^/%CONTAINER%$" 2^>nul') do set "EXISTS=%%i"
if not defined EXISTS (
    echo [INFO] No '%CONTAINER%' container exists. Nothing to stop.
    exit /b 0
)

echo [INFO] Stopping '%CONTAINER%'...
docker stop %CONTAINER% >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Failed to stop '%CONTAINER%'.
    exit /b 1
)

echo [OK] Stopped '%CONTAINER%'.
exit /b 0
