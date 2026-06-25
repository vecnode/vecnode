@echo off
setlocal EnableExtensions

REM ---------------------------------------------------------------------------
REM clear_stirling_pdf_image.bat
REM Remove the Stirling-PDF Docker image to free disk space (forces a fresh
REM download on the next open). The container is removed first if present.
REM ---------------------------------------------------------------------------

set "IMAGE=stirlingtools/stirling-pdf:latest"
set "CONTAINER=stirling-pdf"

where docker >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Docker is not available or not in PATH.
    exit /b 1
)

set "EXISTS="
for /f "delims=" %%i in ('docker ps -aq --filter "name=^/%CONTAINER%$" 2^>nul') do set "EXISTS=%%i"
if defined EXISTS (
    echo [INFO] Removing container '%CONTAINER%' first...
    docker rm -f %CONTAINER% >nul 2>nul
)

docker image inspect %IMAGE% >nul 2>nul
if errorlevel 1 (
    echo [INFO] Image '%IMAGE%' is not present. Nothing to clear.
    exit /b 0
)

echo [INFO] Removing image '%IMAGE%'...
docker rmi %IMAGE% >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Failed to remove image '%IMAGE%'.
    exit /b 1
)

echo [OK] Removed image '%IMAGE%'.
exit /b 0
