@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM Unblock scripts: strip Zone.Identifier (Mark of the Web) from all .bat
REM and .sh files under ./scripts/ so Windows Defender stops flagging them as
REM untrusted internet downloads. Unblock-File does not require admin rights -
REM any user can unblock files they own. Safe and idempotent.
REM ---------------------------------------------------------------------------
powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-ChildItem -Path '%~dp0scripts' -Filter '*.bat' -Recurse -ErrorAction SilentlyContinue | Unblock-File -ErrorAction SilentlyContinue; Get-ChildItem -Path '%~dp0scripts' -Filter '*.sh' -Recurse -ErrorAction SilentlyContinue | Unblock-File -ErrorAction SilentlyContinue" >nul 2>nul

REM Always run relative to this file's directory (repo root expected).
pushd "%~dp0" >nul 2>nul
if errorlevel 1 (
	echo [ERROR] Unable to enter script directory.
	pause
	exit /b 1
)

where cargo >nul 2>nul
if errorlevel 1 (
	echo [ERROR] cargo not found in PATH.
	echo Install Rust first: https://rustup.rs/
	popd >nul
	pause
	exit /b 1
)

set "RUST_HOST="
for /f "tokens=1,* delims=:" %%A in ('rustc -vV ^| findstr /B /C:"host:"') do set "RUST_HOST=%%B"
for /f "tokens=* delims= " %%H in ("%RUST_HOST%") do set "RUST_HOST=%%H"

if not defined RUST_HOST (
	echo [ERROR] Unable to detect rustc host target.
	echo Run "rustc -vV" and ensure Rust is installed correctly.
	popd >nul
	pause
	exit /b 1
)

set "VN_BIN=.\cli\target\%RUST_HOST%\debug\vn.exe"

tasklist /FI "IMAGENAME eq vn.exe" 2>nul | find /I "vn.exe" >nul
if not errorlevel 1 (
	echo [INFO] Detected an existing vn.exe process. Skipping rebuild to avoid file lock.
) else (
	echo [INFO] Building vn CLI for host target %RUST_HOST%...
	cargo build --manifest-path cli/Cargo.toml -p vn --target "%RUST_HOST%"
	if errorlevel 1 (
		echo [ERROR] Build failed.
		popd >nul
		pause
		exit /b 1
	)
)

if not exist "%VN_BIN%" (
	echo [ERROR] Binary not found: %VN_BIN%
	popd >nul
	pause
	exit /b 1
)

echo [INFO] Launching vn...
echo [INFO] Starting vecnode tray icon...
start "vecnode tray" /MIN "%VN_BIN%" tray --repo-root "%~dp0"

"%VN_BIN%" %*
set "VN_EXIT=%ERRORLEVEL%"

popd >nul
if not "%VN_EXIT%"=="0" (
	echo [ERROR] vn exited with code %VN_EXIT%.
	pause
)

exit /b %VN_EXIT%
