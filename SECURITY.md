# vecnode Rust CLI - Security Policy

**Last Updated:** May 14, 2026  
**Version:** 0.1

This document outlines the security practices, deployment guidelines, and threat model for the vecnode Rust CLI. It is designed to be safe for deployment on fresh, untrusted machines while remaining lightweight and user-friendly for development workflows.

---

## Table of Contents

1. [Rust-Specific Security Practices](#rust-specific-security-practices)
2. [Deployment Safety Guidelines](#deployment-safety-guidelines)
3. [Additional Resources](#additional-resources)


---

## Rust-Specific Security Practices

vecnode CLI follows Rust security best practices to minimize memory-safety issues.

### No Unsafe Code

The vecnode CLI is written in 100% safe Rust. We do not use `unsafe` code blocks, which eliminates entire classes of memory-safety vulnerabilities.

**Verification:**
```bash
grep -r "unsafe" cli/crates/vn/src/
# Returns: (empty — no unsafe code)
```

### Dependency Auditing

We use automated tools to audit dependencies for known vulnerabilities:

- **`cargo audit`:** Scans `Cargo.lock` against the RustSec Advisory Database
- **`cargo deny`:** Blocks high-risk crates (unmaintained, incompatible licenses, etc.)


### Pinned Dependencies

- `Cargo.lock` is committed to version control (not ignored)
- All builds use `cargo build --locked` to ensure reproducible versions
- Transitive dependencies are pinned to exact versions

### Dependency List

Current dependencies and their purposes:

| Crate | Version | Purpose | Security Notes |
|-------|---------|---------|-----------------|
| `anyhow` | 1.x | Error handling | Actively maintained, no `unsafe` |
| `clap` | 4.x | CLI argument parsing | Actively maintained, audited |
| `sysinfo` | 0.28.x | System information | Actively maintained |
| `tokio` | 1.x | Async runtime | Actively maintained, audited |
| `reqwest` | 0.11.x | HTTP client (Ollama support, deferred) | Actively maintained, TLS support |
| `serde`/`serde_json`/`toml` | Latest | Config serialization | Actively maintained, audited |
| `crossterm` | Latest | Terminal UI (deferred) | Actively maintained |
| `ratatui` | Latest | TUI library (deferred) | Actively maintained |
| `chrono` | Latest | Timestamp handling | Actively maintained |
| `dirs` | Latest | Config/data paths | Actively maintained |

---

## Deployment Safety Guidelines

### Supported Platforms

- **Linux** (glibc, musl): x86_64, aarch64
- **Windows**: x86_64 (10, 11, Server 2016+)

### Privilege Requirements

**The vecnode CLI does NOT require root or admin privileges for normal operation.**

However, certain subcommands may require elevated privileges:

| Command | Privilege | Reason |
|---------|-----------|--------|
| `vn sys info` | None | Read system information (available to all users) |
| `vn docker ps\|up\|down` | Docker daemon access* | Requires membership in `docker` group (Linux) or Docker Desktop (macOS/Windows) |
| `vn git sync\|status` | None (user repos only) | Only operates on repositories you own or have access to |
| `vn run <script>` | Depends on script | Scripts may install packages (requires sudo) or modify system state |

*On Linux, add your user to the `docker` group: `sudo usermod -aG docker $USER`

### Filesystem Access

The CLI reads and writes files only in well-defined locations:

| Path (Unix) | Path (Windows) | Permission | Purpose |
|-------------|----------------|-----------|---------|
| `~/.config/vn/config.toml` | `%APPDATA%\vn\config.toml` | `0o600` (user only) | CLI configuration (host, model, prompts) |
| `~/.local/share/vn/sessions/` | `%APPDATA%\vn\sessions\` | `0o700` (user only) | Session history files (JSON format) |
| `$VECNODE_REPO_ROOT/scripts/` | `%VECNODE_REPO_ROOT%\scripts\` | Read-only | Script templates (sourced from repo) |
| System temp (`/tmp`, `%TEMP%`) | System temp | Inherited | Temporary files created by scripts (if needed) |

**Important:** Session history is stored in plaintext JSON. Treat session files as sensitive; do not commit them to version control or share them if they contain sensitive information.

### Network Access

By default, the CLI makes **no outbound network connections**.

| Feature | Default | Network Access | Protocol |
|---------|---------|----------------|----------|
| `vn ai` | Stubbed (prints "todo") | None (deferred) | — |
| `vn docker` | Requires Docker daemon | Optional (if Docker configured remotely) | Unix socket or TLS |
| `vn git` | Requires Git | Yes (pulls from Git remote) | HTTPS/SSH (Git-controlled) |
| `vn run` | Depends on script | Depends on script content | — |
| `vn sys` | Local only | None | — |

**Note:** When AI functionality is re-enabled, it will connect to Ollama on `http://127.0.0.1:11434` by default (localhost only). Remote hosts will require explicit configuration and will enforce HTTPS.



---

## Additional Resources

- [Rust Secure Code Working Group](https://github.com/rust-secure-code/wg)
- [RustSec Advisory Database](https://rustsec.org/)
- [OWASP: Secure Coding Practices](https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/)
- [GitHub Security: Binary Authorization & Supply Chain](https://github.com/security)

