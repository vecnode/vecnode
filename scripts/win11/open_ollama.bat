@echo off
setlocal EnableExtensions

tasklist /FI "IMAGENAME eq ollama.exe" 2>nul | find /I "ollama.exe" >nul
if not errorlevel 1 (
    echo [INFO] Ollama is already running.
    exit /b 0
)

if exist "%LOCALAPPDATA%\Programs\Ollama\ollama.exe" (
    start "" "%LOCALAPPDATA%\Programs\Ollama\ollama.exe"
    echo [INFO] Ollama launch requested.
    exit /b 0
)

if exist "C:\Program Files\Ollama\ollama.exe" (
    start "" "C:\Program Files\Ollama\ollama.exe"
    echo [INFO] Ollama launch requested.
    exit /b 0
)

echo [ERROR] Ollama executable not found. Is it installed?
exit /b 1
