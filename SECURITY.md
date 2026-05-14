# Vecnode Rust CLI - Security Policy

**Last Updated:** May 14, 2026  
**Version:** 0.1

This document outlines the security practices, deployment guidelines, and threat model for the vecnode Rust CLI. It is designed to be safe for deployment on fresh, untrusted machines while remaining lightweight and user-friendly for development workflows.

---

## Table of Contents

1. [Rust-Specific Security Practices](#rust-specific-security-practices)
2. [Deployment Safety Guidelines](#deployment-safety-guidelines)
3. [Configuration & Secrets](#configuration--secrets)
4. [Threat Model & Risk Assessment](#threat-model--risk-assessment)
5. [Additional Resources](#additional-resources)


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

## Configuration & Secrets

### File Locations

- **Config file:** `~/.config/vn/config.toml` (Unix) or `%APPDATA%\vn\config.toml` (Windows)
- **Sessions:** `~/.local/share/vn/sessions/` (Unix) or `%APPDATA%\vn\sessions\` (Windows)

### File Permissions (Unix)

The CLI automatically sets restrictive permissions on creation:

- **Config file:** `0o600` (user read/write only; others have no access)
- **Config directory:** `0o700` (user access only; others cannot list or access)
- **Session files:** `0o600` (user read/write only)
- **Session directory:** `0o700` (user access only)

**Manual verification:**
```bash
ls -la ~/.config/vn/config.toml
# Expected: -rw------- (or rw-------)

ls -ld ~/.config/vn/
# Expected: drwx------ (or rwx------)
```

### File Permissions (Windows)

Permissions are inherited from your user profile in `%APPDATA%`. Windows NTFS ACLs are automatically restrictive (only your user account has access). No additional action is required.

### Secrets Management

**Best Practice:** Secrets (API tokens, auth headers) should never be stored in plaintext.

#### Environment Variables (Recommended)

When Ollama functionality is re-enabled, use environment variables for authentication:

```bash
# Define secrets in environment (not command line)
export OLLAMA_AUTH_TOKEN="your-secret-token"
export OLLAMA_HOST="http://ollama-server:11434"

# Run CLI (secrets not visible in process list)
vn ai "your prompt"
```

**Why:** Environment variables are:
- Not visible in shell history
- Not visible in process listings (ps/Get-Process)
- Easier to rotate via CI/CD secrets management
- Supported by most CLI frameworks

#### Configuration File Secrets (Not Recommended)

If you must store secrets in `~/.config/vn/config.toml`, ensure:

1. File permissions are `0o600` (Unix) — the CLI enforces this
2. File is not backed up to cloud storage (iCloud, OneDrive, Google Drive)
3. File is not committed to version control (add to `.gitignore`)
4. File is on an encrypted disk (full disk encryption or LUKS)

**Better approach:** Use OS keyring integration (future enhancement):
```toml
[secrets]
use_keyring = true  # Store tokens in OS keyring instead of plaintext
```

---

## Threat Model & Risk Assessment

### Threat Scenarios

#### Scenario 1: Malicious Ollama Server (When AI Re-enabled)

**Threat:** User connects to an attacker-controlled Ollama server which exfiltrates prompts or injects malicious responses.

**Mitigations:**
- Default: Localhost-only connection (`http://127.0.0.1:11434`)
- Future: Remote hosts must use HTTPS (enforce secure transport)
- Best Practice: Verify Ollama server is running locally before using `vn ai`
- Recommendation: Use `--offline` mode if data must not leave the machine

#### Scenario 2: Malicious Script in Cloned Repo

**Threat:** Attacker creates a vecnode fork with malicious scripts; user clones fork and runs `vn run`.

**Mitigations:**
- Script names are hardcoded (no dynamic loading)
- Repo root must contain `.git` directory to be recognized as valid
- Warning displayed if `VECNODE_REPO_ROOT` env var fails validation
- Best Practice: Clone from official repository: `https://github.com/vecnode/vecnode.git`
- Verification: Check commit hash matches official release tags

#### Scenario 3: Privilege Escalation via Script Execution

**Threat:** User runs `vn run` script that executes with elevated privileges (via sudo).

**Mitigations:**
- CLI does not require root/admin for normal operation
- Scripts document their privilege requirements in headers/comments
- Best Practice: Audit scripts before running, especially after updates
- Recommendation: Run in container or separate user account if untrusted

#### Scenario 4: Config File World-Readable on Shared System

**Threat:** Other users on shared multi-user system read plaintext config/sessions.

**Mitigations:**
- Config and session files created with `0o600`/`0o700` (Unix only)
- Windows relies on NTFS ACLs (user profile access only)
- CLI warns if permissions are too permissive (future enhancement)
- Best Practice: Verify permissions after installation: `ls -la ~/.config/vn/`

#### Scenario 5: Man-in-the-Middle Attack on Git Operations

**Threat:** Attacker intercepts Git pull and injects malicious code.

**Mitigations:**
- Git protocol enforcement is left to OS/Git configuration
- Best Practice: Use SSH keys with GitHub (configured at OS level)
- Recommendation: Enable commit signing and verification



---

## Additional Resources

- [Rust Secure Code Working Group](https://github.com/rust-secure-code/wg)
- [RustSec Advisory Database](https://rustsec.org/)
- [OWASP: Secure Coding Practices](https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/)
- [GitHub Security: Binary Authorization & Supply Chain](https://github.com/security)

