@echo off
setlocal EnableExtensions

for %%I in ("%~dp0..\..") do set "REPO_ROOT=%%~fI"
set "DOCKERFILE_PATH=%REPO_ROOT%\docs\Dockerfile"
set "BUILD_CONTEXT=%REPO_ROOT%\docs"
set "IMAGE_NAME=vecnode-docs:latest"
set "CONTAINER_NAME=vecnode-docs"
set "DOCS_PORT="
set "DOCS_URL="
set "BUILD_LOG=%TEMP%\vecnode_docs_build.log"

echo [INFO] Repository root: %REPO_ROOT%
echo [INFO] Docs Dockerfile: %DOCKERFILE_PATH%
echo [INFO] Build context: %BUILD_CONTEXT%

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

if not exist "%DOCKERFILE_PATH%" (
    echo [ERROR] Docs Dockerfile not found: %DOCKERFILE_PATH%
    exit /b 1
)

echo [INFO] Building docs image...
docker build -t %IMAGE_NAME% -f "%DOCKERFILE_PATH%" "%BUILD_CONTEXT%" >"%BUILD_LOG%" 2>&1
if errorlevel 1 (
    echo [ERROR] Failed to build docs image.
    type "%BUILD_LOG%"
    exit /b 1
)

echo [INFO] Recreating docs container: %CONTAINER_NAME%
docker rm -f %CONTAINER_NAME% >nul 2>nul

docker run -d --name %CONTAINER_NAME% -p 127.0.0.1:3000:3000 %IMAGE_NAME% >nul 2>nul
if errorlevel 1 (
    echo [WARNING] Local port 3000 is unavailable. Falling back to another localhost port.
    docker rm -f %CONTAINER_NAME% >nul 2>nul
    docker run -d --name %CONTAINER_NAME% -P %IMAGE_NAME% >nul
    if errorlevel 1 (
        echo [ERROR] Failed to start docs container.
        exit /b 1
    )

    for /f "tokens=2 delims=:" %%P in ('docker port %CONTAINER_NAME% 3000/tcp 2^>nul') do (
        set "DOCS_PORT=%%P"
        goto :port_found
    )

    if not defined DOCS_PORT (
        echo [ERROR] Could not determine mapped docs port.
        exit /b 1
    )
) else (
    set "DOCS_PORT=3000"
)

:port_found
set "DOCS_URL=http://localhost:%DOCS_PORT%"

echo [INFO] Docs container started.
echo [INFO] Open docs at: %DOCS_URL%
echo [INFO] Opening %DOCS_URL%
start "vecnode docs" "%DOCS_URL%"

exit /b 0