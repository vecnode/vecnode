@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM run_silverbullet.bat
REM Run SilverBullet using Docker latest image.
REM
REM Usage:
REM   run_silverbullet.bat
REM
REM Requirements (Windows):
REM   - docker
REM ---------------------------------------------------------------------------

REM ---------------------------------------------------------------------------
REM DOCKER CHECK ^& SETUP
REM ---------------------------------------------------------------------------

cls



echo [INFO] Checking for required tools.
if not exist "%ProgramFiles%\Docker\Docker\resources\bin\docker.exe" (
    where docker >nul 2>nul
    if errorlevel 1 (
        echo [ERROR] Docker is not available or not in PATH
        echo.
        echo Docker is required to run this script.
        echo Please install Docker Engine/Desktop from:
        echo   https://docs.docker.com/engine/install/
        exit /b 1
    )
)

docker --version >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Docker is not available or not in PATH
    echo.
    echo Docker is required to run this script.
    echo Please install Docker Engine/Desktop from:
    echo   https://docs.docker.com/engine/install/
    exit /b 1
)

for /f "tokens=*" %%i in ('docker --version') do set "DOCKER_VERSION=%%i"
echo [OK] %DOCKER_VERSION%

docker ps >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Docker daemon is not running
    echo.
    echo Please start Docker and try again.
    exit /b 1
)

echo [OK] Docker daemon is running.
echo.

REM ---------------------------------------------------------------------------
REM SILVERBULLET SPACE SETUP
REM ---------------------------------------------------------------------------

set "SB_SPACE_PATH=%USERPROFILE%\silverbullet-space"

if not exist "%SB_SPACE_PATH%" (
    echo [INFO] Space folder does not exist, creating it.
    mkdir "%SB_SPACE_PATH%" >nul 2>nul
    if errorlevel 1 (
        echo [ERROR] Failed to create space folder: %SB_SPACE_PATH%
        exit /b 1
    )
    echo [OK] Created: %SB_SPACE_PATH%
) else (
    echo [OK] Space folder exists: %SB_SPACE_PATH%
)

echo.

REM ---------------------------------------------------------------------------
REM BACKUP SPACE FOLDER TO DESKTOP
REM ---------------------------------------------------------------------------

echo.
echo [INFO] Backing up space folder to Desktop.

set "BACKUP_BASE_PATH=%USERPROFILE%\Desktop"

for /f "tokens=*" %%i in ('powershell -NoProfile -Command "Get-Date -Format yyyyMMdd-HHmmss"') do set "BACKUP_TS=%%i"
set "BACKUP_TARGET=%BACKUP_BASE_PATH%\silverbullet-space-backup-%BACKUP_TS%"

mkdir "%BACKUP_TARGET%" >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Failed to create backup folder: %BACKUP_TARGET%
    exit /b 1
)

robocopy "%SB_SPACE_PATH%" "%BACKUP_TARGET%" /E >nul
if errorlevel 8 (
    echo [ERROR] Backup failed.
    exit /b 1
) else (
    echo [OK] Backup completed: %BACKUP_TARGET%
)

echo.

REM ---------------------------------------------------------------------------
REM DOCKER CONTAINER SETUP ^& RUN
REM ---------------------------------------------------------------------------

echo [INFO] Stopping any existing SilverBullet container.
docker rm -f silverbullet >nul 2>nul

echo [INFO] Starting SilverBullet container from latest image.
echo [INFO] SilverBullet will be available at http://localhost:3000
echo.

docker run -d --rm --name silverbullet -p 127.0.0.1:3000:3000 -v "%SB_SPACE_PATH%:/space" -e SB_USER="user:password" ghcr.io/silverbulletmd/silverbullet:latest >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Docker run failed
    exit /b 1
)

echo [OK] Container started: silverbullet
echo [INFO] Open: http://localhost:3000
echo [INFO] Username: user
echo [INFO] Password: password
echo [INFO] Data folder: %SB_SPACE_PATH%
echo [INFO] Stop with: docker stop silverbullet
echo [INFO] Logs with: docker logs -f silverbullet
endlocal
exit /b 0
