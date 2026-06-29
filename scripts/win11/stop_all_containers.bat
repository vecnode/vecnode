@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM stop_all_containers.bat
REM Stop every running Docker container (without removing them).
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

echo [INFO] Stopping all running containers...
set "HAS_RUNNING="
for /f "usebackq delims=" %%C in (`docker ps -q 2^>nul`) do (
    set "HAS_RUNNING=1"
    docker stop %%C >nul 2>nul
)

if not defined HAS_RUNNING (
    echo [INFO] No running containers to stop.
) else (
    echo [OK] All running containers stopped.
)

exit /b 0
