# Global Windows 11 Configs
# Run as current user - no hardcoded usernames, HKCU: maps to whoever is running this.

# ---------------------------------------------------------------------------
# Disable key logging and transmission to Microsoft (TIPC)
# TIPC (Text Intelligence Platform Client) collects what you type and sends it
# to Microsoft for Bing, autocomplete, and typing suggestions. Setting
# Enabled = 0 stops the collection and upload.
# ---------------------------------------------------------------------------
$tipcPath = "HKCU:\SOFTWARE\Microsoft\Input\TIPC"
if (!(Test-Path $tipcPath)) {
    New-Item -Path $tipcPath -Force | Out-Null
}
Set-ItemProperty -Path $tipcPath -Name "Enabled" -Value 0 -Type DWord
Write-Host "[INFO] Disabled TIPC key logging transmission."

# ---------------------------------------------------------------------------
# Opt out of language list exposure to websites
# Windows can pass your Accept-Language list to websites via the Windows API.
# This is a fingerprinting vector — your specific language order is a stable
# signal trackers use to identify you across sites even without cookies.
# Note: does not affect browser-level Accept-Language headers.
# ---------------------------------------------------------------------------
$langProfilePath = "HKCU:\Control Panel\International\User Profile"
Set-ItemProperty -Path $langProfilePath -Name "HttpAcceptLanguageOptOut" -Value 1 -Type DWord
Write-Host "[INFO] Opted out of language list exposure to websites."

# ---------------------------------------------------------------------------
# Disable suggested content (ads) inside the Windows Settings app
# 338393 - Suggested apps in Settings > Apps
# 338394 - Tips and suggestions in Settings > System > Notifications
# 338396 - Promotional banners inside Settings panels ("Get more out of Windows")
# ---------------------------------------------------------------------------
$cdmPath = "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\ContentDeliveryManager"
Set-ItemProperty -Path $cdmPath -Name "SubscribedContent-338393Enabled" -Value 0 -Type DWord
Set-ItemProperty -Path $cdmPath -Name "SubscribedContent-338394Enabled" -Value 0 -Type DWord
Set-ItemProperty -Path $cdmPath -Name "SubscribedContent-338396Enabled" -Value 0 -Type DWord
Write-Host "[INFO] Disabled suggested content in Settings app."
# vecnode 2026


