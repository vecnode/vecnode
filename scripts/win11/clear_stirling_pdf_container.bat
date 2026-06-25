@echo off
setlocal EnableExtensions

REM ---------------------------------------------------------------------------
REM clear_stirling_pdf_container.bat
REM Remove the Stirling-PDF container (force). The image is left in place, so a
REM later open is fast (no re-download). Use clear_stirling_pdf_image to also
REM remove the image.
REM ---------------------------------------------------------------------------

set "CONTAINER=stirling-pdf"

where docker >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Docker is not available or not in PATH.
    exit /b 1
)

set "EXISTS="
for /f "delims=" %%i in ('docker ps -aq --filter "name=^/%CONTAINER%$" 2^>nul') do set "EXISTS=%%i"
if not defined EXISTS (
    echo [INFO] No '%CONTAINER%' container exists. Nothing to clear.
    exit /b 0
)

echo [INFO] Removing container '%CONTAINER%'...
docker rm -f %CONTAINER% >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Failed to remove container '%CONTAINER%'.
    exit /b 1
)

echo [OK] Removed container '%CONTAINER%'.
exit /b 0
