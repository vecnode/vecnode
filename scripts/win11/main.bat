@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM main.bat
REM Entry point for vecnode - GitHub repository backup tool
REM Usage:
REM   main.bat
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

for %%C in (git curl jq) do (
    where %%C >nul 2>nul
    if errorlevel 1 (
        echo [ERROR] Required command not found: %%C
        echo Please install: git, curl, and jq
        exit /b 1
    )
)

REM ---------------------------------------------------------------------------
REM MAIN MENU - CHOOSE OPERATION
REM ---------------------------------------------------------------------------

:main_menu
echo.
echo What would you like to do?
echo   1 = Backup GitHub
echo   2 = Silverbullet
echo.
set "MAIN_CHOICE="
set /p MAIN_CHOICE="Enter your choice (1 or 2): "

if "%MAIN_CHOICE%"=="1" (
    echo.
    goto :github_header
)

if "%MAIN_CHOICE%"=="2" (
    echo.
    call "%~dp0run_silverbullet.bat"
    exit /b 0
)

echo [ERROR] Invalid choice. Please enter 1 or 2.
goto :main_menu

REM ---------------------------------------------------------------------------
REM GITHUB BACKUP - USERNAME PROMPT
REM ---------------------------------------------------------------------------

:github_header
echo # ============================
echo # vecnode
echo # GitHub Repository Backup
echo # ============================
echo.

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

REM ---------------------------------------------------------------------------
REM GITHUB BACKUP - SOURCE CHOICE
REM ---------------------------------------------------------------------------

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

REM ---------------------------------------------------------------------------
REM COMPLETION
REM ---------------------------------------------------------------------------

:summary
echo.
echo # ============================
echo # Backup process completed
echo # ============================
echo.
endlocal
exit /b 0
