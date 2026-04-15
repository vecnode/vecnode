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
echo.
echo # ============================
echo # SilverBullet Docker Runner
echo # ============================
echo.

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
REM OPTIONAL BACKUP SPACE FOLDER
REM ---------------------------------------------------------------------------

:backup_prompt
echo.
set "BACKUP_CHOICE="
set /p BACKUP_CHOICE="Do you want to backup the space folder elsewhere? (y/n): "

if /i "%BACKUP_CHOICE%"=="y" goto :backup_destination
if /i "%BACKUP_CHOICE%"=="yes" goto :backup_destination
if /i "%BACKUP_CHOICE%"=="n" goto :sync_prompt
if /i "%BACKUP_CHOICE%"=="no" goto :sync_prompt

echo [ERROR] Invalid choice. Please enter 'y' or 'n'.
goto :backup_prompt

:backup_destination
echo.
set "BACKUP_BASE_PATH="
set /p BACKUP_BASE_PATH="Enter backup destination folder path (default: %USERPROFILE%\Desktop): "

if not defined BACKUP_BASE_PATH set "BACKUP_BASE_PATH=%USERPROFILE%\Desktop"

if "%BACKUP_BASE_PATH:~0,1%"=="~" set "BACKUP_BASE_PATH=%USERPROFILE%%BACKUP_BASE_PATH:~1%"

if not exist "%BACKUP_BASE_PATH%" (
    echo [ERROR] Path does not exist: %BACKUP_BASE_PATH%
    goto :backup_destination
)

for /f "tokens=*" %%i in ('powershell -NoProfile -Command "Get-Date -Format yyyyMMdd-HHmmss"') do set "BACKUP_TS=%%i"
set "BACKUP_TARGET=%BACKUP_BASE_PATH%\silverbullet-space-backup-%BACKUP_TS%"

mkdir "%BACKUP_TARGET%" >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Failed to create backup folder: %BACKUP_TARGET%
    goto :backup_destination
)

echo [INFO] Backing up space folder to: %BACKUP_TARGET%
robocopy "%SB_SPACE_PATH%" "%BACKUP_TARGET%" /E >nul
if errorlevel 8 (
    echo [ERROR] Backup failed.
    exit /b 1
) else (
    echo [OK] Backup completed successfully.
)

REM ---------------------------------------------------------------------------
REM OPTIONAL COPY FROM ANOTHER FOLDER
REM ---------------------------------------------------------------------------

:sync_prompt
echo.
set "SYNC_CHOICE="
set /p SYNC_CHOICE="Do you want to copy markdown files from another folder? (y/n): "

if /i "%SYNC_CHOICE%"=="y" goto :sync_source
if /i "%SYNC_CHOICE%"=="n" (
    echo [INFO] Skipping copy.
    goto :after_sync
)

echo [ERROR] Invalid choice. Please enter 'y' or 'n'.
goto :sync_prompt

:sync_source
echo.
set "SOURCE_PATH="
set /p SOURCE_PATH="Enter path to source markdown folder: "

if not defined SOURCE_PATH (
    echo [ERROR] Path cannot be empty.
    goto :sync_source
)

if "%SOURCE_PATH:~0,1%"=="~" set "SOURCE_PATH=%USERPROFILE%%SOURCE_PATH:~1%"

if not exist "%SOURCE_PATH%" (
    echo [ERROR] Path does not exist: %SOURCE_PATH%
    goto :sync_source
)

echo [INFO] Copying markdown files from: %SOURCE_PATH%
set "FOUND_MD="
set /a COPY_COUNT=0

for %%F in ("%SOURCE_PATH%\*.md") do (
    if exist "%%~fF" (
        set "FOUND_MD=1"
        if exist "%SB_SPACE_PATH%\%%~nxF" (
            choice /c YN /n /m "File already exists: %%~nxF. Overwrite? [Y/N]: "
            if errorlevel 2 (
                echo [INFO] Skipped: %%~nxF
            ) else (
                copy /y "%%~fF" "%SB_SPACE_PATH%\%%~nxF" >nul 2>nul
                if not errorlevel 1 set /a COPY_COUNT+=1
            )
        ) else (
            copy "%%~fF" "%SB_SPACE_PATH%\%%~nxF" >nul 2>nul
            if not errorlevel 1 set /a COPY_COUNT+=1
        )
    )
)

if not defined FOUND_MD (
    echo [WARNING] No markdown files found in source folder.
) else (
    echo [OK] Copy step completed. Files copied: !COPY_COUNT!
)

:after_sync

echo.

REM ---------------------------------------------------------------------------
REM DOCKER CONTAINER SETUP ^& RUN
REM ---------------------------------------------------------------------------

echo [INFO] Stopping any existing SilverBullet container.
docker rm -f silverbullet >nul 2>nul

echo [INFO] Starting SilverBullet container from latest image.
echo [INFO] SilverBullet will be available at http://localhost:3000
echo.

docker run -d --rm --name silverbullet -p 3000:3000 -v "%SB_SPACE_PATH%:/space" -e SB_USER="user:password" ghcr.io/silverbulletmd/silverbullet:latest >nul 2>nul
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
