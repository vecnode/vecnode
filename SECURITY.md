# Vecnode Rust CLI - Security Policy

**Last Updated:** May 14, 2026  
**Version:** 1.0

This document outlines the security practices, deployment guidelines, and threat model for the vecnode Rust CLI. It is designed to be safe for deployment on fresh, untrusted machines while remaining lightweight and user-friendly for development workflows.

---

## Table of Contents

1. [Rust-Specific Security Practices](#rust-specific-security-practices)
2. [Deployment Safety Guidelines](#deployment-safety-guidelines)
3. [Configuration & Secrets](#configuration--secrets)
4. [Supply Chain Security](#supply-chain-security)
5. [Threat Model & Risk Assessment](#threat-model--risk-assessment)
6. [Frequently Asked Questions](#frequently-asked-questions)


---

## Rust-Specific Security Practices

Vecnode follows Rust security best practices to minimize memory-safety issues.

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

**These are run on every commit** via CI/CD when configured.

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

## Supply Chain Security

### Binary Verification

Official releases are distributed with checksums and signatures to verify integrity.

#### Verifying Checksums (All Platforms)

```bash
# Download binary and checksums
wget https://github.com/vecnode/vecnode/releases/download/v1.0.0/vn-linux-x86_64
wget https://github.com/vecnode/vecnode/releases/download/v1.0.0/checksums.txt

# Verify checksum (Linux/macOS)
sha256sum -c <(grep "vn-linux-x86_64" checksums.txt)
# Output: vn-linux-x86_64: OK

# Verify checksum (Windows PowerShell)
(Get-FileHash .\vn-windows-x86_64.exe).Hash -eq (Select-String "vn-windows-x86_64" .\checksums.txt).Line
```

#### Verifying Signatures (GPG)

```bash
# Import maintainer's GPG key
gpg --recv-keys <KEY_ID>

# Verify signature
gpg --verify checksums.txt.sig checksums.txt
# Output: Good signature from "Vecnode Maintainers"

# Verify individual binary (after checksum verification)
sha256sum -c checksums.txt
```

### Reproducible Builds

Official releases are reproducible: rebuilding from the same commit produces byte-for-byte identical binaries.

**To verify yourself:**
```bash
git clone https://github.com/vecnode/vecnode
git checkout v1.0.0

cargo build --release --locked
sha256sum target/release/vn

# Compare hash with published checksums.txt
```

### Dependency Audit Reports

CI/CD runs `cargo audit` and `cargo deny` on every commit. Reports are published with each release:

- **Security Advisories:** Any vulnerabilities are disclosed in release notes
- **Dependency Analysis:** Transitive dependencies are audited for unmaintained status
- **License Compliance:** All dependencies comply with permissive licenses (MIT, Apache 2.0, etc.)

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

## Frequently Asked Questions

### Q: Is vecnode safe to run on my machine?

**A:** Yes, for normal development workflows. The CLI:
- Does not require root/admin privileges
- Does not make outbound network connections by default
- Stores config in user-scoped directories with restrictive permissions
- Is written in safe Rust (no memory vulnerabilities)
- Has no hardcoded credentials or backdoors

However, review the [Threat Model](#threat-model--risk-assessment) section for specific concerns about scripts and remote services.

### Q: Can I run vecnode in a container?

**A:** Yes, vecnode is container-friendly:
```bash
docker run -v ~/.config/vn:/root/.config/vn ghcr.io/vecnode/vn ai "prompt"
```

This is recommended if you want additional isolation on untrusted machines.

### Q: What happens if I run an untrusted script with `vn run`?

**A:** The script runs with your privileges (not elevated). The CLI does not provide sandboxing beyond what your OS offers. Best practices:

1. Review the script source code before running
2. Run in a container or VM if the script is untrusted
3. Use a dedicated user account for untrusted scripts

### Q: How do I verify I downloaded a legitimate binary?

**A:** Follow the [Binary Verification](#verifying-checksums-all-platforms) section:
1. Download binary and `checksums.txt` from official release
2. Verify checksum: `sha256sum -c checksums.txt`
3. Verify signature (optional): `gpg --verify checksums.txt.sig`

### Q: Can I use vecnode offline?

**A:** For most commands, yes:
- `vn sys info` — fully offline
- `vn docker` — requires Docker daemon (may be local or remote)
- `vn git` — requires network if pulling from remote; local-only operations work offline
- `vn ai` — currently stubbed; when re-enabled, will have `--offline` mode

### Q: What data does vecnode collect?

**A:** None. The CLI does not phone home, does not send telemetry, and does not collect usage data. All data is stored locally on your machine.

### Q: How do I securely uninstall vecnode?

**A:** Remove the binary and config directory:

```bash
# Remove binary
sudo rm /usr/local/bin/vn

# Remove config (Unix)
rm -rf ~/.config/vn ~/.local/share/vn/sessions

# Remove config (Windows)
rmdir /s %APPDATA%\vn
```

---

## Additional Resources

- [Rust Secure Code Working Group](https://github.com/rust-secure-code/wg)
- [RustSec Advisory Database](https://rustsec.org/)
- [OWASP: Secure Coding Practices](https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/)
- [GitHub Security: Binary Authorization & Supply Chain](https://github.com/security)

---

## Version History

- **1.0** (May 14, 2026): Initial security policy; Ollama functionality stubbed
- Future: Ollama re-enabled with hardened network policy, offline mode, remote host validation