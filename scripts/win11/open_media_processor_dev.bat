@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM open_media_processor_dev.bat
REM Run media-processor container in dev mode with live-mounted UI files.
REM ---------------------------------------------------------------------------

for %%I in ("%~dp0..\..") do set "REPO_ROOT=%%~fI"
set "DOCKERFILE_PATH=%REPO_ROOT%\docker\media-processor\Dockerfile"
set "BUILD_CONTEXT=%REPO_ROOT%"
set "IMAGE_NAME=vecnode-media-processor:latest"
set "CONTAINER_NAME=vecnode-media-processor"
set "UI_PORT=8085"
set "API_PORT=8086"
set "PRESENTATION_PORT=8087"
set "HOST_DESKTOP_WIN=%USERPROFILE%\Desktop"
set "HOST_DESKTOP_CONTAINER=/host/Desktop"
set "HOST_UI_WIN=%REPO_ROOT%\docker\media-processor\ui"
set "HOST_UI_CONTAINER=/app/docker/media-processor/ui"

cls
echo [INFO] Repository root: %REPO_ROOT%
echo [INFO] Dockerfile: %DOCKERFILE_PATH%
echo [INFO] Image: %IMAGE_NAME%
echo [INFO] Container: %CONTAINER_NAME% (dev)
echo [INFO] Host Desktop: %HOST_DESKTOP_WIN%
echo [INFO] Host UI mount: %HOST_UI_WIN%
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

if not exist "%HOST_UI_WIN%" (
	echo [ERROR] UI folder not found: %HOST_UI_WIN%
	exit /b 1
)

if not exist "%HOST_DESKTOP_WIN%" (
	echo [INFO] Creating Desktop folder: %HOST_DESKTOP_WIN%
	mkdir "%HOST_DESKTOP_WIN%"
	if errorlevel 1 (
		echo [ERROR] Unable to create Desktop folder: %HOST_DESKTOP_WIN%
		exit /b 1
	)
)

echo [INFO] Checking image availability...
docker image inspect %IMAGE_NAME% >nul 2>nul
if errorlevel 1 (
	echo [INFO] Image not found. Building media-processor image...
	docker build -t %IMAGE_NAME% -f "%DOCKERFILE_PATH%" "%BUILD_CONTEXT%" 2>&1
	if errorlevel 1 (
		echo [ERROR] Docker build failed.
		exit /b 1
	)
) else (
	echo [INFO] Reusing existing image: %IMAGE_NAME%
)

echo.
echo [INFO] Removing previous container if present...
docker rm -f %CONTAINER_NAME% >nul 2>nul

echo [INFO] Starting media-processor container (dev mode)...
docker run -d --rm --name %CONTAINER_NAME% -p %UI_PORT%:8085 -p %API_PORT%:8086 -p %PRESENTATION_PORT%:8087 -e HOST_DESKTOP_DIR=%HOST_DESKTOP_CONTAINER% -v "%HOST_DESKTOP_WIN%:%HOST_DESKTOP_CONTAINER%" -v "%HOST_UI_WIN%:%HOST_UI_CONTAINER%" %IMAGE_NAME% >nul
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
	echo [OK] media-processor ^(dev^) is ready.
) else (
	echo [WARNING] API health check did not pass in time. Container may still be starting.
)

echo [INFO] UI:  http://localhost:%UI_PORT%
echo [INFO] API: http://localhost:%API_PORT%
echo [INFO] Presentation: http://localhost:%PRESENTATION_PORT%
echo [INFO] Dev mount active: %HOST_UI_WIN% ^> %HOST_UI_CONTAINER%
echo [INFO] Edit HTML/CSS/JS in docker\media-processor\ui and refresh browser.
echo [INFO] Output folder base: %HOST_DESKTOP_WIN%
echo [INFO] Logs: docker logs -f %CONTAINER_NAME%
echo [INFO] Stop: docker stop %CONTAINER_NAME%

start "" "http://localhost:%UI_PORT%"

exit /b 0