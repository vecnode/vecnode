@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM check_internet.bat
REM Network diagnostics for vecnode settings menu.
REM
REM Performs multi-signal internet checks and prints compact I/O counters.
REM ---------------------------------------------------------------------------

echo.
echo # ============================
echo # Internet Diagnostics
echo # ============================

set "ADAPTER_STATE=DOWN"
set "RX_BYTES=NA"
set "TX_BYTES=NA"
set "PING_OK=0"
set "DNS_OK=0"
set "INTERNET_STATUS=OFF"

set "NET_TMP=%TEMP%\vecnode-net-%RANDOM%-%RANDOM%.txt"
powershell -NoProfile -Command "$adapters=Get-NetAdapter -ErrorAction SilentlyContinue | Where-Object { $_.Status -eq 'Up' }; $anyUp=if($adapters){'UP'}else{'DOWN'}; $stats=Get-NetAdapterStatistics -ErrorAction SilentlyContinue; if($stats){$rx=[int64](($stats|Measure-Object -Property ReceivedBytes -Sum).Sum); $tx=[int64](($stats|Measure-Object -Property SentBytes -Sum).Sum)} else {$rx='NA'; $tx='NA'}; Write-Output ('ADAPTER_STATE='+$anyUp); Write-Output ('RX_BYTES='+$rx); Write-Output ('TX_BYTES='+$tx)" > "%NET_TMP%" 2>nul

if exist "%NET_TMP%" (
    for /f "usebackq tokens=1,* delims==" %%K in ("%NET_TMP%") do (
        if /i "%%K"=="ADAPTER_STATE" set "ADAPTER_STATE=%%L"
        if /i "%%K"=="RX_BYTES" set "RX_BYTES=%%L"
        if /i "%%K"=="TX_BYTES" set "TX_BYTES=%%L"
    )
    del /q "%NET_TMP%" >nul 2>nul
)

ping -n 1 -w 1500 1.1.1.1 >nul 2>nul
if not errorlevel 1 set "PING_OK=1"

nslookup www.microsoft.com >nul 2>nul
if not errorlevel 1 set "DNS_OK=1"

if "%PING_OK%"=="1" if "%DNS_OK%"=="1" set "INTERNET_STATUS=ON"

echo.
if "%INTERNET_STATUS%"=="ON" (
    echo [OK] Internet status: ON
) else (
    echo [ERROR] Internet status: OFF
)

if /i "%ADAPTER_STATE%"=="UP" (
    echo [INFO] Network adapter state: at least one adapter is UP
) else (
    echo [WARNING] Network adapter state: no active adapter detected
)

if "%PING_OK%"=="1" (
    echo [INFO] Reachability test ^(ICMP 1.1.1.1^): PASS
) else (
    echo [INFO] Reachability test ^(ICMP 1.1.1.1^): FAIL
)

if "%DNS_OK%"=="1" (
    echo [INFO] DNS test ^(www.microsoft.com^): PASS
) else (
    echo [INFO] DNS test ^(www.microsoft.com^): FAIL
)

echo.
echo [INFO] Small I/O summary ^(all adapters combined^):
if /i "%RX_BYTES%"=="NA" (
    echo [WARNING] Unable to read RX/TX byte counters.
) else (
    echo [INFO] RX bytes: %RX_BYTES%
    echo [INFO] TX bytes: %TX_BYTES%
)

echo.
endlocal
exit /b 0

