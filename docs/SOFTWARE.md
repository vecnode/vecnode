# vecnode TUI Buttons

The vecnode TUI is a small dashboard of button-style menu items.

## Dashboard

These are the root buttons shown when `vn` starts:

- `vn ai "prompt"` - placeholder for the local AI flow
- `vn sys info` - prints system information
- `vn run ubuntu22` - opens the Ubuntu 22 action menu
- `vn run win11` - opens the Windows 11 action menu

## Ubuntu 22 Menu

When running on non-Windows hosts, the Ubuntu menu exposes:

- `vn run ubuntu22-check-internet`
- `vn run ubuntu22-check-dependencies`
- `vn run ubuntu22-download-all-repos`
- `vn run ubuntu22-download-all-orgs`
- `vn run ubuntu22-run-cli-container`
- `vn run ubuntu22-run-silverbullet`
- `< Back to Dashboard`

## Windows 11 Menu

When running on Windows hosts, the Windows menu exposes:

- `vn run win11-check-internet`
- `vn run win11-check-dependencies`
- `vn run win11-download-all-repos`
- `vn run win11-open-docker`
- `vn run win11-open-docs`
- `vn run win11-check-docker`
- `vn run win11-run-silverbullet`
- `vn run win11-install-apps`
- `< Back to Dashboard`

## Windows 11 Apps Submenu

The apps submenu currently includes:

- `vn run win11-install-app-wezterm`
- `< Back to win11`

## Controls

- `Tab` switches focus between the dashboard and the input area.
- `Up` and `Down` move between buttons while the dashboard is focused.
- `Enter` runs the selected button or sends input from the text field.
- `,` and `.` page through the CLI output.
- `q` or `Esc` exits the TUI.

