@echo off
setlocal EnableExtensions EnableDelayedExpansion

echo [Local Network Scan]
echo.

rem --- Get local IP (first IPv4 address) ---
set "LOCAL_IP="
for /f "tokens=2 delims=:" %%A in ('ipconfig ^| findstr /R /C:"IPv4 Address"') do (
    if "!LOCAL_IP!"=="" (
        set "LOCAL_IP=%%A"
        set "LOCAL_IP=!LOCAL_IP: =!"
    )
)

if "!LOCAL_IP!"=="" (
    echo [ERROR] Could not determine local IP address.
    exit /b 1
)

echo [INFO] Local IP: !LOCAL_IP!

for /f "tokens=1-3 delims=." %%A in ("!LOCAL_IP!") do set "SUBNET=%%A.%%B.%%C"

echo [INFO] Scanning !SUBNET!.1-254
echo.

set /a FOUND=0
for /l %%i in (1,1,254) do (
    ping -n 1 -w 300 !SUBNET!.%%i >nul 2>&1
    if not errorlevel 1 (
        set /a FOUND+=1
        echo   !SUBNET!.%%i
    )
)

echo.
if !FOUND! EQU 0 (
    echo [INFO] No reachable hosts found on !SUBNET!.0/24
) else (
    echo [INFO] !FOUND! host^(s^) reachable on !SUBNET!.0/24
    echo.
    echo [INFO] ARP cache for subnet ^(includes resolved hostnames^):
    arp -a 2>nul | findstr " !SUBNET!"
)

exit /b 0
