@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM run_silverbullet.bat
REM Build and run SilverBullet using Docker from a specified directory
REM
REM Usage:
REM   Double-click this file, or run: run_silverbullet.bat
REM   Prompts for the SilverBullet repository folder path
REM
REM Requirements (Windows):
REM   - Docker (docker command)
REM ---------------------------------------------------------------------------

cls
echo.
echo # ============================
echo # SilverBullet Docker Runner
REM echo # ============================
echo.

REM Check for Docker
echo [INFO] Checking for required tools.

docker --version >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Docker is not available or not in PATH
    echo.
    echo Docker is required to run this script.
    echo Please install Docker Desktop from:
    echo   https://www.docker.com/products/docker-desktop
    echo.
    echo After installing Docker Desktop, restart your terminal/PowerShell.
    pause
    exit /b 1
)

for /f "tokens=*" %%i in ('docker --version') do set "DOCKER_VERSION=%%i"
echo [OK] %DOCKER_VERSION%

REM Check if Docker daemon is running
docker ps >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Docker daemon is not running
    echo.
    echo Please start Docker Desktop and try again.
    echo   - On Windows: Search for "Docker Desktop" in Start menu and open it
    echo   - Wait for Docker to finish starting ^(you'll see it in the system tray^)
    echo.
    pause
    exit /b 1
)

echo [OK] Docker daemon is running.
echo.

:prompt_path
echo.
set "SILVERBULLET_PATH="
set /p SILVERBULLET_PATH="Enter path to SilverBullet repository: "

if not defined SILVERBULLET_PATH (
    echo [ERROR] Path cannot be empty.
    goto :prompt_path
)

REM Remove trailing backslash if present
if "!SILVERBULLET_PATH:~-1!"=="\" set "SILVERBULLET_PATH=!SILVERBULLET_PATH:~0,-1!"

REM Validate path exists
if not exist "!SILVERBULLET_PATH!" (
    echo [ERROR] Path does not exist: !SILVERBULLET_PATH!
    goto :prompt_path
)

REM Validate it's a SilverBullet repo
if not exist "!SILVERBULLET_PATH!\Dockerfile" (
    echo [ERROR] This doesn't appear to be a SilverBullet repository.
    echo [ERROR] Missing: Dockerfile in !SILVERBULLET_PATH!
    goto :prompt_path
)

echo [OK] Valid SilverBullet repository detected.
echo.

:prompt_space
set "SB_SPACE_PATH="
set /p SB_SPACE_PATH="Enter path to SilverBullet space folder: "

if not defined SB_SPACE_PATH (
    echo [ERROR] Space folder path cannot be empty.
    goto :prompt_space
)

REM Remove trailing backslash if present
if "!SB_SPACE_PATH:~-1!"=="\" set "SB_SPACE_PATH=!SB_SPACE_PATH:~0,-1!"

if not exist "!SB_SPACE_PATH!" (
    echo [INFO] Space folder does not exist, creating it.
    mkdir "!SB_SPACE_PATH!" >nul 2>nul
    if errorlevel 1 (
        echo [ERROR] Failed to create space folder: !SB_SPACE_PATH!
        pause
        exit /b 1
    )
)

set "SB_PORT=3000"
set /p SB_PORT="Enter host port (default 3000): "
if not defined SB_PORT set "SB_PORT=3000"

:docker_build
echo [INFO] Building Docker image.
echo.

docker build -t silverbullet:local "!SILVERBULLET_PATH!" 
if errorlevel 1 (
    echo [ERROR] Docker build failed
    pause
    exit /b 1
)

echo [OK] Docker image built successfully.
echo.

:docker_run
echo [INFO] Starting SilverBullet container.
echo [INFO] SilverBullet will be available at http://localhost:!SB_PORT!
echo.

REM Remove any previous container with the same name for clean reruns
docker rm -f silverbullet-local >nul 2>nul

REM Run detached so the script returns to terminal while server keeps running.
REM We override the image entrypoint to avoid Windows line-ending issues in docker-entrypoint.sh.
docker run -d --name silverbullet-local -p !SB_PORT!:3000 --mount type=bind,source="!SB_SPACE_PATH!",target=/space --entrypoint /silverbullet silverbullet:local /space
if errorlevel 1 (
    echo [ERROR] Docker run failed
    pause
    exit /b 1
)

echo [OK] Container started: silverbullet-local
echo [INFO] Open: http://localhost:!SB_PORT!
echo [INFO] Data folder: !SB_SPACE_PATH!
echo [INFO] Stop with: docker stop silverbullet-local
echo [INFO] Logs with: docker logs -f silverbullet-local

endlocal
exit /b 0
