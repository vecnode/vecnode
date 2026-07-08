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

Genuinely OS-specific work (dependency installs, ollama setup, dotfiles, starting
Docker Desktop) is delegated to per-OS scripts under `scripts/ubuntu22/*.sh` and
`scripts/win11/*.bat`, which `vn run` locates and executes. Everything cross-platform —
port scanning (`vn net`), and all Docker app launching/maintenance (`vn app`,
`vn docker`) — is implemented natively in Rust so there is one code path, not two.

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
        apps.rs            vn app open|stop|list — native Docker app engine (see below)
        ollama/session.rs  Chat session persistence (per-session message history)
        sys.rs             vn sys info|update|clean (uses sysinfo)
        docker.rs          vn docker ps|up|down|prune|check|stop-all|remove-*
        git.rs             vn git sync|status
        net.rs             vn net scan — RustScan-based open-port scan (native Rust)
        run.rs             vn run <name> — maps script names to scripts/** and executes
        mcp.rs             vn mcp serve [--http] — MCP server entrypoint (see below)
      mcp/                 MCP (Model Context Protocol) host: see "MCP host" below
        mod.rs             Module list
        approval.rs        ApprovalGate — the confirm-before-destructive-tool-call bridge
        apps_toolset.rs     AppsToolset — the "Apps" toolset (list/open/stop apps)
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
changes**; the TUI is hard to exercise headlessly. CI
([.github/workflows/ci.yml](.github/workflows/ci.yml)) runs `cargo fmt --check`, `clippy
-D warnings`, and `cargo build --locked` on Ubuntu and Windows on every push/PR to
`main`, plus `cargo deny` and `cargo audit` — but it doesn't replace running these
locally before you push, since the TUI itself still needs a manual smoke test.

## How the TUI dispatches work

`tui/app.rs` defines a static menu tree (`MenuKind` + `menu_items`). Each leaf is a
`CommandItem` whose `Action::Execute(args)` re-invokes the **current `vn` executable**
as a child process with those args (e.g. `["run", "win11-check-internet"]` or
`["net", "scan"]`), piping stdout/stderr into the "CLI Output" panel. So adding a TUI
action = add a `CommandItem` pointing at an existing `vn` subcommand. Spawned commands
run via the `command-group` crate (`group_spawn()`), so each owns its own process
group/job object — killing one (or quitting the TUI) also kills anything it spawned
(e.g. a `docker build`), instead of leaving it orphaned.

Destructive menu items use `Action::ExecuteConfirm` instead of `Action::Execute`: it
arms the input box with `InputPurpose::ConfirmDestructive` and only runs the command if
you type `yes`. Use it for anything that stops/removes containers or images; regular
commands stay `Action::Execute`.

Keybindings beyond Up/Down/Enter: `x` kills every running background process without
quitting the TUI; `e` scrolls the CLI Output panel to the previous error/stderr line;
`Esc` steps back one menu level (or cancels the input box), only quitting from the root
Dashboard — `q` still quits immediately from anywhere in the Dashboard. Mouse wheel
scrolls CLI Output; left-clicking a dashboard row selects and runs it. `**bold**` markers
in CLI Output text (common in AI replies) render bold via `spans_with_bold`.

**Nothing that can block should run on the render loop.** Spawned commands, the Ollama
chat worker, and the embedded MCP server each own a dedicated OS thread (or tokio runtime)
and talk back to `event_loop` only through channels drained non-blockingly
(`pump_process`/`pump_mcp_approvals`/`pump_docker_panel`); `docker ps` for the Docker panel
follows the same pattern (`refresh_docker_panel` spawns a thread, `pump_docker_panel`
applies the result when it arrives) so a slow/hung Docker daemon can't stall the whole
TUI. If you add a new background action, follow this pattern rather than calling a
blocking operation directly from `event_loop`.

`commands/run.rs` `map_script()` is a match from a script name to a relative path
under `scripts/`. To add a script-backed task: drop the script in the right OS folder,
add a `map_script` arm, and wire a `CommandItem` for it. Menu items are OS-gated by
`menu_allowed_on_current_os`. Docker-app names are intercepted first by `try_native()`
and routed to `commands/apps.rs` — scripts are only for genuinely OS-specific work
(package installs, ollama, dotfiles, Docker Desktop startup).

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
- `vn ai models [--tools-only]` — list installed model names (one per line; used by the
  TUI). `--tools-only` checks each model's reported `capabilities` via `ollama show` and
  skips ones without `"tools"` (see `model_supports_tools`) — the TUI's chat always
  attaches tools (see below), so a model without that capability can't chat at all.
- `vn ai pull <name>` — download a model (e.g. `llama3.2`, which is small and supports
  tools).
- `vn ai chat "<message>" [--model m] [--session s]` — send a message; context is kept
  per session via [ollama/session.rs](cli/crates/vn/src/ollama/session.rs) so turns
  build on each other across invocations.

In the TUI's AI submenu (win11-ai / ubuntu22-ai): **Select Model** opens a dynamic menu
listing installed **tool-capable** models only (the TUI shells out to `vn ai models
--tools-only` and builds rows from the output; the chosen model shows in the header and
is passed as `--model`). If none are installed, the menu shows a placeholder recommending
`vn ai pull llama3.2`. **Download Model** and **Chat** arm the input box (see
`InputPurpose`) so the next typed line is
routed to `vn ai pull` / `vn ai chat`. Output streams into the CLI Output panel and the
session log file like any other command. The Ollama *server* itself still needs to be
installed/running — that is what the `check-ollama` / `open-ollama` scripts handle.

## Dockerized apps (Open menu) — native `vn app`

The TUI **Open** submenu launches/controls self-hosted web apps in Docker through **one
cross-platform Rust code path**: [cli/crates/vn/src/commands/apps.rs](cli/crates/vn/src/commands/apps.rs).
There are **no per-OS launch scripts anymore** — the old `run_*/stop_*` `.bat`/`.sh` pairs
were replaced by:

- `vn app open <name> [--no-open]` — check Docker, build (local apps) or pull the image,
  create/start the container, wait for the port, open **Chrome** (default-browser fallback).
- `vn app stop <name>` — stop the container (kept for fast reopen where applicable).
- `vn app list` — the app registry.
- `vn docker check|stop-all|remove-containers|remove-images` — global maintenance.

Legacy names still work: `vn run win11-open-library-portal` etc. route to the native
implementation (see `try_native` in `run.rs`), so old muscle memory and docs don't break.

Every app is an `AppPlan` in `plan_for()` — image, container, optional build context,
lifecycle (`Recreate` vs `Reuse` running/stopped), ports, env, mounts, readiness port.
**To add an app: add one `plan_for` arm and a `CommandItem` in both Open submenus.**
The engine enforces the security posture automatically (don't bypass it): ports published
**loopback-only** (`127.0.0.1`), `--cap-drop ALL`, `--security-opt no-new-privileges`,
`--pids-limit` for locally built images, and on Linux `--user $(id -u):$(id -g)` where the
plan sets `linux_user` (so bind-mount files stay user-owned). See SECURITY.md.

Apps: SilverBullet (`ghcr.io/silverbulletmd/silverbullet`, port 3000, backs up the space
folder to Desktop before each start), Stirling-PDF (`stirlingtools/stirling-pdf`, port 8080,
reuses its container), docs (mdBook, port 3000), plus the locally built **library-portal**
(8090), **doc-processor** (8085/8086, image source `docker/media-processor/`) and
**media-downloader** (8095), which rebuild + recreate on every open (picks up code edits).

**media-downloader (custom, locally built):** a tiny yt-dlp + ffmpeg web app in
[docker/media-downloader/](docker/media-downloader/) — `debian:12-slim` + a single stdlib
`app.py`. `yt-dlp` is installed via `pip` (the Debian apt package is years stale and breaks
against current sites). Paste a video URL, pick **MP3 / WAV / MP4**; the file is saved to the
host **Desktop**, bind-mounted at `/output` by `vn app open media-downloader` (port 8095). Because it fetches arbitrary web links, it is hardened: the container runs **non-root**
(Linux uses `--user $(id -u):$(id -g)`), with **`--cap-drop ALL`**, **`--security-opt
no-new-privileges`** and a pids limit; the app accepts only http/https URLs and rejects
hosts that resolve to loopback/private/link-local up front. Beyond that initial check,
`yt-dlp` itself is pointed at a small **in-container egress-guard proxy**
(`start_egress_proxy` in `app.py`, `--proxy http://127.0.0.1:$EGRESS_PROXY_PORT`) that
re-resolves and re-validates every connection yt-dlp makes at actual connect time — this
is what catches a redirect to a different (private) host or DNS rebinding between the
initial check and yt-dlp's own lookup, since the fast-fail check alone only ever sees the
first URL. yt-dlp also runs with `--ignore-config --restrict-filenames --max-filesize`,
and saves via a sanitized, traversal-checked, collision-safe filename confined to the
mount.

**library-portal (custom, locally built):** a lightweight viewer/manager for the repo's
`library/` folder, living in [docker/library-portal/](docker/library-portal/) —
`python:3.12-slim` + a single stdlib `app.py`, plus PyMuPDF for thumbnails.
`vn app open library-portal` builds the image (the build context is only
`docker/library-portal/`, so **no PDFs enter the image**), then runs it with the repo
`library/` bind-mounted on port 8090 and opens Chrome. The server walks `/library` per
request and renders an index in the same light, simple card style as doc-processor's UI
(shared `--bg`/`--surface`/`--accent` tokens, no serif headings), streaming PDFs inline for
the browser viewer.
It supports edit/rename, per-document tags, delete, list/grid/tree views, sort, and (in tree
view) creating folders and drag-and-drop moving PDFs into them. App state
(metadata overrides + tags) is kept in `library/.portal/portal.json` and thumbnails are
cached under `library/.portal/thumbs/` (both gitignored, hidden from the listing); PDFs are
only modified on an explicit rename. `open` rebuilds + recreates the container each time
(picks up `app.py` edits).

Note on `.bat`: inside an `if (…) else (…)` block, any literal `(`/`)` in an `echo`
must be escaped as `^(`/`^)` (or avoided) — an unescaped `)` closes the block early and
cmd fails with "… was unexpected at this time."

## MCP host

vecnode is an MCP ([Model Context Protocol](https://modelcontextprotocol.io)) **host**: it
exposes its own host-control functions as MCP tools, using the official
[`rmcp`](https://crates.io/crates/rmcp) Rust SDK. v1 has one toolset, **Apps**
([cli/crates/vn/src/mcp/apps_toolset.rs](cli/crates/vn/src/mcp/apps_toolset.rs)):
`list_apps`, `open_app`, `stop_app` — thin wrappers around `commands::apps::{list, open_reported, stop_reported}`,
so there is exactly one implementation whether the caller is an external MCP client or
vecnode's own Ollama chat.

**Reaching it:**
- `vn mcp serve` — stdio transport, for MCP clients that spawn their own subprocess
  (Claude Desktop/Code's usual local-server config). Runs headless: no TUI is attached.
- `vn mcp serve --http [--port 7332]` — loopback-only Streamable HTTP transport, same
  headless behavior.
- **The TUI's embedded server**: on startup, `tui/app.rs` spawns the same HTTP transport
  on its own thread (`spawn_mcp_server`), bound to `127.0.0.1:7332` by default. The "MCP
  Server" panel shows its status and tool count. This is the one that gets a real approval
  prompt (see below), since the TUI is attached.

**Approval gate** ([cli/crates/vn/src/mcp/approval.rs](cli/crates/vn/src/mcp/approval.rs)):
`stop_app` is destructive, so it calls `ApprovalGate::request()` before acting. With the
TUI attached, this arms the input box with the same "type yes to confirm" pattern used for
destructive menu items (`InputPurpose::ApproveMcp`) and blocks the tool call until you
answer. Headless (`vn mcp serve`), `ApprovalGate::headless()` auto-denies every request —
fail-closed, since there's no console free to prompt on (stdio *is* the protocol channel).
`list_apps`/`open_app` aren't gated; they run immediately, matching the TUI's own
"vn app open" menu items.

**Ollama chat integration** (`spawn_chat_worker` in
[tui/app.rs](cli/crates/vn/src/tui/app.rs)): every chat turn attaches `ToolInfo` entries
built dynamically from `AppsToolset::list_tools()` (the same JSON-schema metadata an MCP
client sees via `tools/list` — `ollama-rs`'s `ToolInfo`/`ToolFunctionInfo` are plain public
structs, so no compile-time-known tool types are needed). If the reply has `tool_calls`,
each one is dispatched via `AppsToolset::call_by_name` (in-process, no HTTP/stdio
round-trip — `stop_app` still goes through the same `ApprovalGate`), logged into the CLI
Output panel as `[MCP] Calling ...` / `[MCP] Result: ...`, and fed back as a
`ChatMessage::tool(...)` — capped at `MAX_TOOL_ROUNDS` (4) to bound a confused model's
tool-calling loop.

**To add a tool:** add a `#[tool(description = "...")]` method (with a params struct
deriving `serde::Deserialize` + `schemars::JsonSchema` if it takes arguments) to
`AppsToolset` (or a new toolset struct, composed alongside it in `commands/mcp.rs` and
`tui/app.rs`'s `spawn_mcp_server`), and add a matching arm in `call_by_name` so the Ollama
integration can dispatch to it by name.

## Conventions

- Match the surrounding style: `anyhow::Result` with `.context(...)`, `cfg!`/`#[cfg]`
  gating for OS-specific code, no panics in command paths.
- Keep cross-platform behavior in mind — almost everything must work on both Windows
  and Linux. Windows-only code (tray, windows-sys) is `#[cfg(target_os = "windows")]`.
- Scripts come in matched `.sh` (ubuntu22) / `.bat` (win11) pairs; when you add or
  change one, consider the other.
- Don't commit `cli/target/` (gitignored). Commit/push only when asked.
