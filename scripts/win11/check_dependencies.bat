@echo off
REM ---------------------------------------------------------------------------
REM check_dependencies.bat
REM Comprehensive dependency checker and installer for vecnode CLI.
REM
REM Checks for: git, curl, jq, docker
REM Offers automatic installation if any are missing.
REM
REM Usage:
REM   check_dependencies.bat
REM ---------------------------------------------------------------------------

setlocal EnableExtensions EnableDelayedExpansion

echo.
echo # ============================
echo # vecnode CLI Dependencies
echo # ============================
echo.

REM Initialize variables
set "DEPENDENCIES=git curl jq docker"
set /a MISSING_COUNT=0
set "MISSING_LIST="

REM ---------------------------------------------------------------------------
REM DEPENDENCY CHECK PHASE
REM ---------------------------------------------------------------------------

echo Checking for required dependencies...
echo.

for %%D in (%DEPENDENCIES%) do (
    set "DEP=%%D"
    set "FOUND=0"
    set "VERSION="
    
    echo [Checking !DEP!]
    
    where !DEP! >nul 2>nul
    if !errorlevel! equ 0 (
        set "FOUND=1"
        
        if "!DEP!"=="git" (
            for /f "tokens=*" %%V in ('git --version 2^>nul') do set "VERSION=%%V"
            echo   [OK] !VERSION!
        ) else if "!DEP!"=="curl" (
            for /f "tokens=*" %%V in ('curl --version 2^>nul ^| findstr /R "."') do (
                set "VERSION=%%V"
                goto :curl_version_done
            )
            :curl_version_done
            echo   [OK] !VERSION!
        ) else if "!DEP!"=="jq" (
            for /f "tokens=*" %%V in ('jq --version 2^>nul') do set "VERSION=%%V"
            echo   [OK] !VERSION!
        ) else if "!DEP!"=="docker" (
            docker ps >nul 2>nul
            if !errorlevel! equ 0 (
                for /f "tokens=*" %%V in ('docker --version 2^>nul') do set "VERSION=%%V"
                echo   [OK] !VERSION!
            ) else (
                echo   [WARNING] Found but daemon may not be running
                set "FOUND=1"
            )
        )
    ) else (
        echo   [MISSING]
        set /a MISSING_COUNT+=1
        set "MISSING_LIST=!MISSING_LIST! !DEP!"
    )
    echo.
)

REM ---------------------------------------------------------------------------
REM SUMMARY ^& INSTALLATION PROMPT
REM ---------------------------------------------------------------------------

if !MISSING_COUNT! equ 0 (
    echo [SUCCESS] All dependencies are installed!
    echo.
    exit /b 0
)

echo [WARNING] The following dependencies are missing or not accessible:
for %%M in (!MISSING_LIST!) do (
    echo   - %%M
)
echo.

:install_prompt
set "INSTALL_CHOICE="
set /p INSTALL_CHOICE="Would you like to install the missing dependencies? (y/n): "

if /i "!INSTALL_CHOICE!"=="y" (
    goto :install_phase
) else if /i "!INSTALL_CHOICE!"=="n" (
    echo.
    echo [INFO] Skipping installation.
    exit /b 0
) else (
    echo [ERROR] Invalid choice. Please enter 'y' or 'n'.
    goto :install_prompt
)

REM ---------------------------------------------------------------------------
REM INSTALLATION PHASE
REM ---------------------------------------------------------------------------

:install_phase
echo.
echo # ============================
echo # Installing Dependencies
echo # ============================
echo.

where winget >nul 2>nul
if !errorlevel! equ 0 (
    echo [INFO] Using winget package manager
    echo.
    
    for %%M in (!MISSING_LIST!) do (
        if "%%M"=="docker" (
            echo [INFO] Installing Docker...
            winget install -e --id Docker.DockerDesktop >nul 2>nul
            if !errorlevel! equ 0 (
                echo [OK] Docker installed
            ) else (
                echo [WARNING] Docker installation may require manual steps
            )
        ) else (
            echo [INFO] Installing %%M...
            winget install -e --id %%M >nul 2>nul
            if !errorlevel! equ 0 (
                echo [OK] %%M installed
            ) else (
                echo [WARNING] %%M installation may require manual action
            )
        )
    )
) else (
    echo [INFO] winget not found. Installation requires manual action or alternative package manager.
    echo.
    echo Please install the missing dependencies:
    for %%M in (!MISSING_LIST!) do (
        if "%%M"=="docker" (
            echo   - Docker: https://docs.docker.com/engine/install/
        ) else if "%%M"=="git" (
            echo   - Git: https://git-scm.com/download/win
        ) else if "%%M"=="curl" (
            echo   - Curl: https://curl.se/download.html
        ) else if "%%M"=="jq" (
            echo   - jq: https://stedolan.github.io/jq/download/
        )
    )
    exit /b 1
)

REM ---------------------------------------------------------------------------
REM VERIFICATION PHASE
REM ---------------------------------------------------------------------------

echo.
echo # ============================
echo # Verifying Installation
echo # ============================
echo.

set /a VERIFICATION_FAILED=0

for %%M in (!MISSING_LIST!) do (
    set "DEP=%%M"
    
    where !DEP! >nul 2>nul
    if !errorlevel! equ 0 (
        if "!DEP!"=="git" (
            for /f "tokens=*" %%V in ('git --version 2^>nul') do set "VERSION=%%V"
            echo   Verifying !DEP!... [OK] !VERSION!
        ) else if "!DEP!"=="curl" (
            for /f "tokens=*" %%V in ('curl --version 2^>nul ^| findstr /R "."') do (
                set "VERSION=%%V"
                goto :verify_curl_done
            )
            :verify_curl_done
            echo   Verifying !DEP!... [OK] !VERSION!
        ) else if "!DEP!"=="jq" (
            for /f "tokens=*" %%V in ('jq --version 2^>nul') do set "VERSION=%%V"
            echo   Verifying !DEP!... [OK] !VERSION!
        ) else if "!DEP!"=="docker" (
            for /f "tokens=*" %%V in ('docker --version 2^>nul') do set "VERSION=%%V"
            echo   Verifying !DEP!... [OK] !VERSION!
        )
    ) else (
        echo   Verifying !DEP!... [FAILED]
        set /a VERIFICATION_FAILED=1
    )
)

echo.

if !VERIFICATION_FAILED! equ 0 (
    echo [SUCCESS] All dependencies verified successfully!
    echo.
    exit /b 0
) else (
    echo [ERROR] Some dependencies failed verification. Please try manual installation.
    echo.
    exit /b 1
)

endlocal
