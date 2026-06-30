# AGENTS.md — vecnode `vn` CLI

Guidance for AI coding agents (and humans) working in this repository. `CLAUDE.md`
points here so both toolchains read the same instructions.

## What this is

`vecnode` is a personal cross-platform CLI named **`vn`**, used to manage and run
day-to-day functions on the author's Windows 11 and Ubuntu 22.04 machines. The core
is a Rust binary that exposes:

- A **TUI dashboard** (the default when `vn` runs with no subcommand) for picking and
  running tasks interactively.
- **Subcommands** (`vn sys`, `vn docker`, `vn git`, `vn net`, `vn run`, `vn ai`, …)
  that the TUI shells out to, or that you can run directly.
- A **Windows system-tray agent** (`vn tray`) that stays resident as a small icon and
  spawns new elevated TUI terminals on demand.

Most heavy lifting (network checks, docker, github sync, ollama, etc.) is delegated to
per-OS scripts under `scripts/ubuntu22/*.sh` and `scripts/win11/*.bat`, which `vn run`
locates and executes. Newer, performance-sensitive work (e.g. open-port scanning) is
implemented natively in Rust instead.

## How it runs (the resident-tray model)

On Windows the intended flow is:

1. Double-click **`run_cli.bat`** — **no administrator rights required**. It unblocks the
   scripts, builds `vn`, installs `rustscan` if missing, then launches a **tray agent**
   (`vn tray`, minimized/hidden) and a first (non-elevated) TUI.
2. The tray agent runs persistently as a shield icon. Its menu has **"Open TUI Terminal"**
   (normal, same privilege as the tray — the default), **"Open Admin TUI Terminal"**
   (re-launches `vn` elevated via `ShellExecuteW "runas"`, triggering a UAC prompt), and
   **"Quit"**.
3. You open as many TUIs as you want from the tray; closing them leaves the tray
   running. Quitting from the tray exits everything.

Run everyday tasks from a normal (non-admin) TUI. Only use **"Open Admin TUI Terminal"**
for commands that actually need elevation (e.g. some Docker / doc-processor workflows);
that way `run_cli.bat` never has to be launched as Administrator.

A single-instance lock (`%TEMP%\vecnode.vn.tray.lock`) keeps only one tray alive.
Linux has no tray; use `./run_cli.sh` to build and launch the TUI directly.

## Layout

```
run_cli.bat / run_cli.sh   Build + launch (Windows installs the tray; both auto-install rustscan)
cli/                       Cargo workspace
  Cargo.toml               Workspace + shared dependency versions
  crates/vn/
    Cargo.toml             The vn binary; tray-item + windows-sys are Windows-only deps
    src/
      main.rs              clap CLI definition + dispatch
      config.rs            TOML config at <config_dir>/vn/config.toml (ollama, sessions, prompts)
      tray.rs              Windows tray agent (cfg-gated; bails on non-Windows)
      tui/app.rs           The whole TUI: menu tree, process spawning, log/docker panels
      commands/
        mod.rs             Module list
        ai.rs              vn ai status|models|pull|chat (Ollama via the ollama-rs crate)
        ollama/session.rs  Chat session persistence (per-session message history)
        sys.rs             vn sys info|update|clean (uses sysinfo)
        docker.rs          vn docker ps|up|down|prune
        git.rs             vn git sync|status
        net.rs             vn net scan — RustScan-based open-port scan (native Rust)
        run.rs             vn run <name> — maps script names to scripts/** and executes
scripts/ubuntu22/*.sh      Linux task scripts
scripts/win11/*.bat        Windows task scripts
scripts/tools-cli/alpine/  In-container tools workflow
dotfiles/                  Dotfile setup (e.g. win11 setup_dotfiles.bat)
docs/, docker/, assets/
```

## Build, run, check

```bash
# Build
cargo build --manifest-path cli/Cargo.toml -p vn

# Run the TUI directly (no launcher)
cargo run --manifest-path cli/Cargo.toml -p vn --

# Run a subcommand
cargo run --manifest-path cli/Cargo.toml -p vn -- sys info
cargo run --manifest-path cli/Cargo.toml -p vn -- net scan        # local /24 open ports
cargo run --manifest-path cli/Cargo.toml -p vn -- net scan 10.0.0.5

# Lint / format before finishing a change
cargo clippy --manifest-path cli/Cargo.toml -p vn
cargo fmt --manifest-path cli/Cargo.toml
```

There is no test suite yet. **Always `cargo build` (and ideally `clippy`) after Rust
changes**; the TUI is hard to exercise headlessly.

## How the TUI dispatches work

`tui/app.rs` defines a static menu tree (`MenuKind` + `menu_items`). Each leaf is a
`CommandItem` whose `Action::Execute(args)` re-invokes the **current `vn` executable**
as a child process with those args (e.g. `["run", "win11-check-internet"]` or
`["net", "scan"]`), piping stdout/stderr into the "CLI Output" panel. So adding a TUI
action = add a `CommandItem` pointing at an existing `vn` subcommand.

`commands/run.rs` `map_script()` is a big match from a script name to a relative path
under `scripts/`. To add a script-backed task: drop the script in the right OS folder,
add a `map_script` arm, and wire a `CommandItem` for it. Menu items are OS-gated by
`menu_allowed_on_current_os`.

Repo root is resolved via `--repo-root`, `VECNODE_REPO_ROOT`, the config location, or
by walking up for a dir that has both `.git/` and `scripts/`.

## Networking: open-port scanning

`vn net scan [target]` ([cli/crates/vn/src/commands/net.rs](cli/crates/vn/src/commands/net.rs))
runs [RustScan](https://github.com/bee-san/RustScan) — `rustscan -a <target> --greppable`.
With no target it derives the host's local `/24` (via a non-sending UDP-connect trick)
and scans the whole subnet, which is far faster than the old ping-sweep scanner it
replaced. `--greppable` keeps it self-contained (no nmap hand-off required). RustScan is
a standalone binary, **not** a linkable crate; the launchers and `check_dependencies`
scripts install it with `cargo install rustscan`.

## AI / Ollama

`vn ai` talks to a local [Ollama](https://ollama.com) server through the
[`ollama-rs`](https://crates.io/crates/ollama-rs) crate (a normal Cargo dependency, so
it is compiled in automatically — no separate install like RustScan). Subcommands:

- `vn ai status` — is the Ollama server reachable?
- `vn ai models` — list installed model names (one per line; used by the TUI).
- `vn ai pull <name>` — download a model (e.g. `llama3.2`).
- `vn ai chat "<message>" [--model m] [--session s]` — send a message; context is kept
  per session via [ollama/session.rs](cli/crates/vn/src/ollama/session.rs) so turns
  build on each other across invocations.

In the TUI's AI submenu (win11-ai / ubuntu22-ai): **Select Model** opens a dynamic menu
listing installed models (the TUI shells out to `vn ai models` and builds rows from the
output; the chosen model shows in the header and is passed as `--model`); **Download
Model** and **Chat** arm the input box (see `InputPurpose`) so the next typed line is
routed to `vn ai pull` / `vn ai chat`. Output streams into the CLI Output panel and the
session log file like any other command. The Ollama *server* itself still needs to be
installed/running — that is what the `check-ollama` / `open-ollama` scripts handle.

## Dockerized apps (Open menu)

The TUI **Open** submenu launches/controls self-hosted web apps in Docker. Each app is a
matched pair of `scripts/win11/*.bat` + `scripts/ubuntu22/*.sh`, wired through `run.rs`
and `menu_items`. The menu is for opening/stopping apps only — image/container cleanup is
handled globally by the Docker submenu's "remove containers" / "remove images". Per app:

- **open** (e.g. `run_stirling_pdf`): check Docker, then run/start the container (the
  image is pulled on first run), wait for the port to respond, and open the web UI in
  **Chrome** (falling back to the default browser). Reuses a running container and
  `docker start`s a stopped one.
- **stop**: `docker stop` the container but keep it (fast reopen).

Most apps pull a published image (SilverBullet `ghcr.io/silverbulletmd/silverbullet` port
3000, Stirling-PDF `stirlingtools/stirling-pdf` port 8080). Several apps are built locally:
**library-portal** (port 8090), **doc-processor** (pandoc→PDF; ports 8085/8086, image source
`docker/media-processor/`), and **media-downloader** (port 8095). To add a pulled-image app,
copy the Stirling-PDF open/stop script pair, change the image/port/container name, add the
`run.rs` mappings, and add the `CommandItem`s to both Open submenus.

**media-downloader (custom, locally built):** a tiny yt-dlp + ffmpeg web app in
[docker/media-downloader/](docker/media-downloader/) — `debian:12-slim` + a single stdlib
`app.py`. `yt-dlp` is installed via `pip` (the Debian apt package is years stale and breaks
against current sites). Paste a video URL, pick **MP3 / WAV / MP4**; the server downloads to a
private temp dir, streams the file back as a browser download, then deletes it (no host mount,
no state). `run_media_downloader.*` builds + runs on port 8095 and opens Chrome.

**library-portal (custom, locally built):** a lightweight viewer/manager for the repo's
`library/` folder, living in [docker/library-portal/](docker/library-portal/) —
`python:3.12-slim` + a single stdlib `app.py`, plus PyMuPDF for thumbnails.
`run_library_portal.*` `docker build`s the image (the build context is only
`docker/library-portal/`, so **no PDFs enter the image**), then runs it with the repo
`library/` bind-mounted on port 8090 and opens Chrome. The server walks `/library` per
request and renders an Anthropic-style index, streaming PDFs inline for the browser viewer.
It supports edit/rename, per-document tags, delete, list/grid/tree views, sort, and (in tree
view) creating folders and drag-and-drop moving PDFs into them. App state
(metadata overrides + tags) is kept in `library/.portal/portal.json` and thumbnails are
cached under `library/.portal/thumbs/` (both gitignored, hidden from the listing); PDFs are
only modified on an explicit rename. `open` rebuilds + recreates the container each time
(picks up `app.py` edits).

Note on `.bat`: inside an `if (…) else (…)` block, any literal `(`/`)` in an `echo`
must be escaped as `^(`/`^)` (or avoided) — an unescaped `)` closes the block early and
cmd fails with "… was unexpected at this time."

## Conventions

- Match the surrounding style: `anyhow::Result` with `.context(...)`, `cfg!`/`#[cfg]`
  gating for OS-specific code, no panics in command paths.
- Keep cross-platform behavior in mind — almost everything must work on both Windows
  and Linux. Windows-only code (tray, windows-sys) is `#[cfg(target_os = "windows")]`.
- Scripts come in matched `.sh` (ubuntu22) / `.bat` (win11) pairs; when you add or
  change one, consider the other.
- Don't commit `cli/target/` (gitignored). Commit/push only when asked.
