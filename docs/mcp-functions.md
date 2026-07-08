# vecnode MCP functions

Human-readable tracker for every tool `vn`'s MCP host exposes. Update this file in the
same PR whenever a tool is added, removed, renamed, or its behavior changes materially.
See [AGENTS.md](../AGENTS.md#mcp-host) for how the toolsets are wired together in code.

Reachable via: `vn mcp serve` (stdio), `vn mcp serve --http` (loopback HTTP), the TUI's
embedded server (same HTTP transport, `127.0.0.1:7332`), and vecnode's own in-TUI Ollama
chat (in-process, no MCP transport).

## Apps toolset (`cli/crates/vn/src/mcp/apps_toolset.rs`)

- `list_apps` — list the Dockerized vecnode apps by name (docs, silverbullet, stirling-pdf, library-portal, media-downloader, doc-processor). Always allowed, no approval needed.
- `open_app` — build/start an app's container and wait for it to become ready; opens a browser tab unless `no_open` says otherwise (see default rules below). Always allowed, no approval needed.
- `stop_app` — stop a running app's container. **Destructive — requires interactive approval in the TUI**; auto-denied with no TUI attached (headless `vn mcp serve`).
- `restart_app` — stop_app + open_app in one call, for "it's stuck, restart it" instead of two tool calls. **Destructive — requires interactive approval in the TUI**; auto-denied with no TUI attached.

`open_app`/`restart_app`'s browser-opening default depends on the caller: headless/external MCP clients default to not opening a browser (`no_open` defaults to `true`); vecnode's own in-TUI chat defaults to opening one (`no_open` defaults to `false`), using Chrome if installed, else the OS default browser. Pass `no_open` explicitly to override either way.

## Docker toolset (`cli/crates/vn/src/mcp/docker_toolset.rs`)

- `list_containers` — list every container docker knows about on this host (running or stopped), with image, state, status, and published ports — not just vecnode's own apps. Read-only, no approval needed.
- `container_logs` — tail a container's stdout+stderr (defaults to the last 50 lines). Read-only, no approval needed.
- `docker_check` — confirm the daemon is running, list running containers (`docker ps`), and report total container/image counts. Read-only, no approval needed.
- `docker_stop_all` — stop *every* running container on the host, not just vecnode's apps. **Destructive — requires interactive approval in the TUI**; auto-denied with no TUI attached.
- `docker_remove_containers` — stop and permanently remove every container on the host. **Destructive/irreversible — requires interactive approval in the TUI**; auto-denied with no TUI attached.
- `docker_remove_images` — permanently remove every docker image on the host. **Destructive/irreversible — requires interactive approval in the TUI**; auto-denied with no TUI attached.
- `disk_usage` — how much disk space images/containers/volumes/build cache are using (wraps `docker system df`). Read-only, no approval needed.

`docker_stop_all`/`docker_remove_containers`/`docker_remove_images` act host-wide (anything docker has, not just vecnode's own containers/images) and are hard or impossible to undo — gated the same way as `stop_app`, unlike the per-app tools which only ever touch vecnode's own known containers.

## Adding a tool

New tools go on an existing `impl AppsToolset { #[tool_router(...)] ... }` block (or a new
one, for a new toolset — see `docker_toolset.rs` as the template). Add a line above, and a
matching `call_by_name`/`call_*_by_name` dispatch arm in the code — see AGENTS.md's
"To add a tool" note.

## Ideas for future toolsets (not yet built)

Rough layering for where this could grow next, loosely ordered by how much they'd need
beyond what already exists in `commands::apps`:

- **App health/status** — an `app_status` tool combining `docker_ps_all` + the per-app `wait_port` from each `AppPlan` to answer "is doc-processor actually up" without opening a browser or re-running the whole `open_app` flow.
- **Library/file browsing** — read-only listing of `library/pdfs/` for the library-portal app, so a model could answer "what PDFs do I have" without a human opening the portal.

None of these are implemented yet — this section is a parking lot for scope, not a
commitment.
