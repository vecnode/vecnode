@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM setup_dotfiles.bat - Copy dotfiles to Windows 11 user profile
REM ---------------------------------------------------------------------------

set "DOTFILES_SOURCE=%~dp0"
set "SSH_CONFIG_SOURCE=%DOTFILES_SOURCE%ssh\config"
set "SSH_CONFIG_DEST=%USERPROFILE%\.ssh\config"

REM Ensure .ssh directory exists
if not exist "%USERPROFILE%\.ssh" (
    mkdir "%USERPROFILE%\.ssh"
    echo [INFO] Created directory: %USERPROFILE%\.ssh
)

REM Backup existing SSH config if it exists
if exist "%SSH_CONFIG_DEST%" (
    set "BACKUP_FILE=%SSH_CONFIG_DEST%.backup_%RANDOM%"
    copy "%SSH_CONFIG_DEST%" "!BACKUP_FILE!" >nul
    echo [INFO] Backed up existing SSH config to: !BACKUP_FILE!
)

REM Copy SSH config from dotfiles to destination
if exist "%SSH_CONFIG_SOURCE%" (
    copy "%SSH_CONFIG_SOURCE%" "%SSH_CONFIG_DEST%" >nul
    echo [INFO] Copied SSH config to: %SSH_CONFIG_DEST%
) else (
    echo [WARNING] SSH config source not found: %SSH_CONFIG_SOURCE%
)

echo [INFO] Dotfiles setup complete.
exit /b 0
