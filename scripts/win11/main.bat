@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM main.bat
REM Entry point for vecnode - GitHub repository backup tool
REM
REM This script prompts for user input and coordinates the backup process.
REM
REM Usage:
REM   Double-click this file, or run: main.bat
REM
REM Requirements (Windows):
REM   - git
REM   - curl
REM   - jq
REM ---------------------------------------------------------------------------

cls
echo.
echo # ============================
echo # vecnode
echo # GitHub Repository Backup
echo # ============================
echo.

REM Check for required commands
for %%C in (git curl jq) do (
    where %%C >nul 2>nul
    if errorlevel 1 (
        echo [ERROR] Required command not found: %%C
        echo Please install: git, curl, and jq
        pause
        exit /b 1
    )
)

:prompt_username
echo.
set "GITHUB_USERNAME="
set /p GITHUB_USERNAME="Enter GitHub username: "

if not defined GITHUB_USERNAME (
    echo [ERROR] GitHub username cannot be empty.
    goto :prompt_username
)

echo [INFO] GitHub username set to: %GITHUB_USERNAME%
echo.

:prompt_source
echo.
echo What would you like to download?
echo   1 = Personal repositories only
echo   2 = Organizations only
echo   3 = Both personal repositories and organizations
echo.
set "SOURCE_CHOICE="
set /p SOURCE_CHOICE="Enter your choice (1, 2, or 3): "

if "%SOURCE_CHOICE%"=="1" (
    echo.
    echo [INFO] Downloading personal repositories for "%GITHUB_USERNAME%"
    echo.
    call "%~dp0download_all_repos.bat" "%GITHUB_USERNAME%"
    goto :summary
)

if "%SOURCE_CHOICE%"=="2" (
    echo.
    echo [INFO] Downloading organization repositories
    echo.
    call "%~dp0download_all_orgs.bat"
    goto :summary
)

if "%SOURCE_CHOICE%"=="3" (
    echo.
    echo [INFO] Downloading personal repositories for "%GITHUB_USERNAME%"
    echo.
    call "%~dp0download_all_repos.bat" "%GITHUB_USERNAME%"
    echo.
    echo [INFO] Downloading organization repositories
    echo.
    call "%~dp0download_all_orgs.bat"
    goto :summary
)

echo [ERROR] Invalid choice. Please enter 1, 2, or 3.
goto :prompt_source

:summary
echo.
echo # ============================
echo # Backup process completed
echo # ============================
echo.
pause
endlocal
exit /b 0
