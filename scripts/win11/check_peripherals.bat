@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM check_peripherals.bat
REM Print all currently connected plug-and-play peripherals, one per line.
REM ---------------------------------------------------------------------------

powershell -NoProfile -Command "Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue | Where-Object { $_.Status -eq 'OK' -or $_.Status -eq 'Unknown' } | ForEach-Object { if ($_.FriendlyName) { $_.FriendlyName } elseif ($_.Name) { $_.Name } } | Where-Object { $_ -and $_.Trim().Length -gt 0 } | Sort-Object -Unique"

exit /b %ERRORLEVEL%
