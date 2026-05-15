@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM open_media_processor.bat
REM Build and run media-processor container with UI and API ports.
REM ---------------------------------------------------------------------------

for %%I in ("%~dp0..\..") do set "REPO_ROOT=%%~fI"
set "DOCKERFILE_PATH=%REPO_ROOT%\docker\media-processor\Dockerfile"
set "BUILD_CONTEXT=%REPO_ROOT%"
set "IMAGE_NAME=vecnode-media-processor:latest"
set "CONTAINER_NAME=vecnode-media-processor"
set "UI_PORT=8085"
set "API_PORT=8086"

cls
echo [INFO] Repository root: %REPO_ROOT%
echo [INFO] Dockerfile: %DOCKERFILE_PATH%
echo [INFO] Image: %IMAGE_NAME%
echo [INFO] Container: %CONTAINER_NAME%
echo.

where docker >nul 2>nul
if errorlevel 1 (
	echo [ERROR] Docker is not available or not in PATH
	exit /b 1
)

docker info >nul 2>nul
if errorlevel 1 (
	echo [ERROR] Docker daemon is not running
	echo [INFO] Start Docker Desktop first, then retry.
	exit /b 1
)

if not exist "%DOCKERFILE_PATH%" (
	echo [ERROR] Dockerfile not found: %DOCKERFILE_PATH%
	exit /b 1
)

echo [INFO] Building media-processor image...
docker build -t %IMAGE_NAME% -f "%DOCKERFILE_PATH%" "%BUILD_CONTEXT%" 2>&1
if errorlevel 1 (
	echo [ERROR] Docker build failed.
	exit /b 1
)

echo.
echo [INFO] Removing previous container if present...
docker rm -f %CONTAINER_NAME% >nul 2>nul

echo [INFO] Starting media-processor container...
docker run -d --rm --name %CONTAINER_NAME% -p %UI_PORT%:8085 -p %API_PORT%:8086 %IMAGE_NAME% >nul
if errorlevel 1 (
	echo [ERROR] Docker run failed.
	exit /b 1
)

echo [INFO] Waiting for API health endpoint...
set "READY=0"
for /L %%N in (1,1,20) do (
	curl --silent --fail http://localhost:%API_PORT%/health >nul 2>nul
	if not errorlevel 1 (
		set "READY=1"
		goto :health_ok
	)
	REM timeout fails in non-interactive redirected stdin sessions (like TUI spawn)
	ping -n 2 127.0.0.1 >nul
)

:health_ok
if "%READY%"=="1" (
	echo [OK] media-processor is ready.
) else (
	echo [WARNING] API health check did not pass in time. Container may still be starting.
)

echo [INFO] UI:  http://localhost:%UI_PORT%
echo [INFO] API: http://localhost:%API_PORT%
echo [INFO] Logs: docker logs -f %CONTAINER_NAME%
echo [INFO] Stop: docker stop %CONTAINER_NAME%

start "" "http://localhost:%UI_PORT%"

exit /b 0
