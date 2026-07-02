# vecnode - Security Policy

**Last Updated:** July 2026
**Version:** 0.1.0

This document describes the security practices and threat model for vecnode: the
`vn` Rust CLI/TUI, its per-OS scripts, and the Dockerized web apps launched from
the TUI's Open menu. It is written to match the code as it exists in this
repository.

---

## Table of Contents

1. [Rust CLI Security Practices](#rust-cli-security-practices)
2. [Script Execution Model](#script-execution-model)
3. [Dockerized Web Apps](#dockerized-web-apps)
4. [Filesystem Access](#filesystem-access)
5. [Network Access](#network-access)
6. [Additional Resources](#additional-resources)

---

## Rust CLI Security Practices

### No Unsafe Code

The `vn` CLI is written in 100% safe Rust; there are no `unsafe` blocks.

**Verification:**
```bash
grep -r "unsafe" cli/crates/vn/src/
# Returns: (empty - no unsafe code)
```

### Dependency Auditing

- **`cargo audit`** scans `Cargo.lock` against the RustSec Advisory Database.
- **`cargo deny`** (see [cli/deny.toml](cli/deny.toml)) warns on unmaintained
  crates and non-permissive licenses, and bans `openssl` in favor of rustls.
- There is currently **no CI**; run both tools manually before releases.

### Pinned Dependencies

- `Cargo.lock` is committed to version control.
- Builds use `cargo build --locked` for reproducible versions.

### Dependency List

Current direct dependencies (see [cli/Cargo.toml](cli/Cargo.toml)):

| Crate | Version | Purpose |
|-------|---------|---------|
| `anyhow` | 1.0 | Error handling |
| `clap` | 4.5 | CLI argument parsing |
| `crossterm` / `ratatui` | 0.28 | Terminal UI |
| `dirs` | 5.0 | Config/data paths |
| `ollama-rs` | 0.3 | Ollama client for `vn ai` |
| `reqwest` | 0.12 (rustls, no default features) | HTTP client |
| `serde` / `serde_json` / `toml` | 1.0 / 1.0 / 0.8 | Config + session serialization |
| `sysinfo` | 0.31 | `vn sys info` |
| `tokio` | 1.40 | Async runtime |
| `chrono` | 0.4 | Timestamps |
| `futures-util` | 0.3 | Streaming |
| `tray-item` / `windows-sys` | 0.10 / 0.48 | Windows-only tray agent |

### Privilege Requirements

**`vn` does NOT require root or admin privileges for normal operation.**
The Windows launcher (`run_cli.bat`) runs unelevated; the tray offers a separate
"Open Admin TUI Terminal" (UAC prompt) only for tasks that need it.

| Command | Privilege | Notes |
|---------|-----------|-------|
| `vn sys info` | None | Read-only system information |
| `vn docker ...` / Open-menu apps | Docker daemon access | `docker` group (Linux) or Docker Desktop (Windows) |
| `vn git sync\|status` | None | Operates on your own repositories |
| `vn net scan` | None | RustScan TCP connect scan (see Network Access) |
| `vn run <script>` | Depends on script | Some scripts install packages (may prompt for sudo/UAC) |

---

## Script Execution Model

`vn run <name>` does **not** execute arbitrary paths or shell strings. Script
names map through a **static allow-list** in
[cli/crates/vn/src/commands/run.rs](cli/crates/vn/src/commands/run.rs) to fixed
relative paths under `scripts/` in the detected repo root (validated to contain
`.git/` + `scripts/`). `.bat` files run via `cmd /C`; `.sh` files run via
`wsl bash` on Windows. UNC paths are rejected for WSL translation. User input is
never interpolated into a command line; `vn docker up|down <name>` validates the
service name against Docker's allowed character set.

---

## Dockerized Web Apps

The TUI Open menu launches local web apps in Docker. Their shared posture:

- **Loopback-only exposure:** every app publishes its ports as
  `-p 127.0.0.1:<port>:<port>`, so nothing is reachable from the LAN. If you
  deliberately expose one, put authentication in front of it first.
- **Locally built apps run as a non-root user** (uid 10001 baked into the
  image; on Linux `vn` passes `--user $(id -u):$(id -g)` so files written to
  bind mounts stay owned by you), with `--cap-drop ALL`,
  `--security-opt no-new-privileges`, and `--pids-limit 512`.
- All of this is enforced by **one Rust code path**
  ([cli/crates/vn/src/commands/apps.rs](cli/crates/vn/src/commands/apps.rs), the
  `vn app open|stop` engine) instead of per-OS scripts, so the posture cannot
  drift between platforms.

Per-app threat model:

| App | Port | Mounts | Notes |
|-----|------|--------|-------|
| library-portal | 8090 | `library/` read-write | Unauthenticated UI that can rename/move/delete PDFs - safe only because it is loopback-bound. State kept in `library/.portal/`. |
| doc-processor | 8085/8086 | host Desktop read-write | pandoc/tectonic markdown-to-PDF + pypdf join; output confined to the Desktop mount. |
| media-downloader | 8095 | host Desktop read-write | Fetches **untrusted URLs** (yt-dlp): http/https only, hosts resolving to loopback/private/link-local ranges are refused (SSRF guard), `--ignore-config --restrict-filenames --no-exec --max-filesize`, sanitized traversal-checked collision-safe output names. Known limit: a public URL redirecting to a private IP is not caught. |
| SilverBullet / Stirling-PDF / docs | 3000/8080 | space/none | Pulled published images. SilverBullet uses a default `SB_USER=user:password` credential - acceptable only while loopback-bound; change it before any wider exposure. |

Image supply chain: base images are pinned tags (`debian:12-slim`,
`python:3.12-slim`); Python deps are version-pinned except `yt-dlp`, which is
deliberately installed latest at build time (stale extractors break against
current sites). tectonic is a pinned release binary downloaded over HTTPS.

---

## Filesystem Access

| Path (Unix) | Path (Windows) | Permission | Purpose |
|-------------|----------------|-----------|---------|
| `~/.config/vn/config.toml` | `%APPDATA%\vn\config.toml` | `0o600` on Unix (Windows inherits profile ACLs) | CLI configuration |
| `~/.local/share/vn/sessions/` | `%LOCALAPPDATA%\vn\sessions\` | `0o700` on Unix | `vn ai` chat session history (plaintext JSON) |
| `<repo>/logs/` | same | user | TUI session logs (gitignored) |
| `<repo>/scripts/` | same | read/execute | Allow-listed task scripts |
| `<repo>/library/` | same | read-write (library-portal only) | PDF library (gitignored, never enters images) |

**Session history and TUI logs are plaintext.** Treat them as sensitive; they
are gitignored - do not commit or share them.

---

## Network Access

| Feature | Default | Network Access |
|---------|---------|----------------|
| `vn ai` | On demand | Ollama at `http://127.0.0.1:11434` (localhost only; other hosts require explicit config) |
| `vn net scan` | On demand | **Actively scans** the local /24 subnet (common ports) or a given target with RustScan - only scan networks you own or have permission to test |
| `vn git sync` | On demand | Git remotes over HTTPS/SSH |
| `vn docker` / Open apps | On demand | Docker daemon; image pulls from Docker Hub/GHCR; media-downloader fetches user-supplied URLs (guarded, see above) |
| `vn sys`, TUI | - | None |

---

## Additional Resources

- [Rust Secure Code Working Group](https://github.com/rust-secure-code/wg)
- [RustSec Advisory Database](https://rustsec.org/)
- [OWASP: Secure Coding Practices](https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/)
