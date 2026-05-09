@echo off
setlocal EnableExtensions EnableDelayedExpansion

docker ps
if errorlevel 1 exit /b 1

set /a CONTAINER_COUNT=0
for /f "usebackq delims=" %%C in (`docker ps -aq`) do (
    set /a CONTAINER_COUNT+=1
)

set /a IMAGE_COUNT=0
for /f "usebackq delims=" %%I in (`docker images -aq`) do (
    set /a IMAGE_COUNT+=1
)

echo Containers: !CONTAINER_COUNT!
echo Images: !IMAGE_COUNT!
exit /b 0
