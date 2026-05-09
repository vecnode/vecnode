@echo off
setlocal EnableExtensions EnableDelayedExpansion

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

echo [INFO] Building vn CLI...
cargo build --manifest-path cli/Cargo.toml -p vn
if errorlevel 1 (
	echo [ERROR] Build failed.
	popd >nul
	pause
	exit /b 1
)

if not exist ".\cli\target\debug\vn.exe" (
	echo [ERROR] Binary not found: .\cli\target\debug\vn.exe
	popd >nul
	pause
	exit /b 1
)

echo [INFO] Launching vn...
".\cli\target\debug\vn.exe" %*
set "VN_EXIT=%ERRORLEVEL%"

popd >nul
if not "%VN_EXIT%"=="0" (
	echo [ERROR] vn exited with code %VN_EXIT%.
	pause
)

exit /b %VN_EXIT%
