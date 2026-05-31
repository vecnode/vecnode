# vecnode TUI Buttons

The vecnode TUI is a small dashboard of button-style menu items.

## Dashboard

These are the root buttons shown when `vn` starts:

- `vn ai "prompt"` - placeholder for the local AI flow
- `vn sys info` - prints system information
- `vn run ubuntu22` - opens the Ubuntu 22 action menu
- `vn run win11` - opens the Windows 11 action menu

## Ubuntu 22 Menu

When running on non-Windows hosts, `vn run ubuntu22` opens grouped submenus:

- `vn run ubuntu22-ai`
- `vn run ubuntu22-network`
- `vn run ubuntu22-dependencies`
- `vn run ubuntu22-github`
- `vn run ubuntu22-docker`
- `vn run ubuntu22-open`
- `< Back to Dashboard`

### Ubuntu 22 AI

- `vn run ubuntu22-check-ollama`
- `vn run ubuntu22-open-ollama`
- `< Back to ubuntu22`

### Ubuntu 22 Network

- `vn run ubuntu22-check-local-network`
- `vn run ubuntu22-check-internet`
- `< Back to ubuntu22`

### Ubuntu 22 Dependencies

- `vn run ubuntu22-check-dependencies`
- `< Back to ubuntu22`

### Ubuntu 22 GitHub

- `vn run ubuntu22-download-all-repos`
- `vn run ubuntu22-download-all-orgs`
- `< Back to ubuntu22`

### Ubuntu 22 Docker

- `vn run ubuntu22-open-docker`
- `vn run ubuntu22-check-docker`
- `vn run ubuntu22-remove-containers`
- `vn run ubuntu22-remove-images`
- `< Back to ubuntu22`

### Ubuntu 22 Open

- `vn run ubuntu22-open-docs`
- `vn run ubuntu22-open-silverbullet`
- `vn run ubuntu22-open-media-processor`
- `< Back to ubuntu22`

## Windows 11 Menu

When running on Windows hosts, `vn run win11` opens grouped submenus:

- `vn run win11-ai`
- `vn run win11-dotfiles`
- `vn run win11-network`
- `vn run win11-dependencies`
- `vn run win11-github`
- `vn run win11-docker`
- `vn run win11-open`
- `< Back to Dashboard`

### Windows 11 AI

- `vn run win11-check-ollama`
- `vn run win11-open-ollama`
- `< Back to win11`

### Windows 11 Dotfiles

- `vn run win11-setup-dotfiles`
- `< Back to win11`

### Windows 11 Network

- `vn run win11-check-peripherals`
- `vn run win11-check-local-network`
- `vn run win11-check-internet`
- `< Back to win11`

### Windows 11 Dependencies

- `vn run win11-check-dependencies`
- `< Back to win11`

### Windows 11 GitHub

- `vn run win11-download-all-repos`
- `< Back to win11`

### Windows 11 Docker

- `vn run win11-open-docker`
- `vn run win11-check-docker`
- `vn run win11-remove-containers`
- `vn run win11-remove-images`
- `< Back to win11`

### Windows 11 Open

- `vn run win11-open-docs`
- `vn run win11-open-silverbullet`
- `vn run win11-open-media-processor`
- `vn run win11-open-media-processor-dev`
- `< Back to win11`

## Controls

- `Tab` switches focus between the dashboard and the input area.
- `Up` and `Down` move between buttons while the dashboard is focused.
- `Enter` runs the selected button or sends input from the text field.
- `,` and `.` page through the CLI output.
- `q` or `Esc` exits the TUI.

