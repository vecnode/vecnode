@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM remove_images.bat
REM Remove all Docker images on the host.
REM ---------------------------------------------------------------------------

where docker >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Docker is not available or not in PATH
    exit /b 1
)

docker info >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Docker daemon is not running
    exit /b 1
)

echo [INFO] Removing all Docker images...
set "HAS_IMAGES="
for /f "usebackq delims=" %%I in (`docker images -aq 2^>nul`) do (
    set "HAS_IMAGES=1"
    docker rmi -f %%I >nul 2>nul
)

if not defined HAS_IMAGES (
    echo [INFO] No images to remove.
) else (
    echo [OK] All images removed.
)

exit /b 0
