@echo off
setlocal EnableExtensions

REM ---------------------------------------------------------------------------
REM stop_papra.bat
REM Stop the Papra container. The container is kept (not removed), so it can be
REM reopened quickly with run_papra.bat. Your library/ and library/.papra-data/
REM are untouched.
REM ---------------------------------------------------------------------------

set "CONTAINER=papra"

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

echo [OK] Stopped '%CONTAINER%'. It is kept and can be reopened.
exit /b 0
