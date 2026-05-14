# vn CLI (Rust)

Cross-platform personal vecnode Rust CLI.

## Goals

- Single Rust binary command surface
- Script delegation to existing shell and batch workflows
- Ratatui interactive mode for terminal-first UX

## Quick Start

```bash
cd cli
cargo build
cargo run -p vn -- --help
```

## Installation

### Local Workspace

```bash
cd cli
cargo build
cargo run -p vn -- --help
```

### Global Binary (from repo)

```bash
cd cli
cargo install --path crates/vn
vn --help
```

### Verify Installation

```bash
vn sys info
```

## Security

The vecnode CLI is designed with security in mind for deployment on fresh, untrusted machines.

**Key Features:**
- ✅ No `unsafe` Rust code
- ✅ No hardcoded secrets or credentials
- ✅ Secure file permissions on config/session files (Unix: 0o600/0o700)
- ✅ Input validation for Docker services and Git paths
- ✅ No outbound network calls by default
- ✅ Dependency auditing via `cargo audit` and `cargo deny`

**For Security Details:**
- Read [../SECURITY.md](../SECURITY.md) for complete security policy
- Read [../SECURITY_REVIEW.md](../SECURITY_REVIEW.md) for implementation details
- Report vulnerabilities privately to [security@vecnode.io](mailto:security@vecnode.io)

## Core Commands

### System

```bash
# System information (OS, CPU, RAM, hostname)
vn sys info
```

### Docker

```bash
vn docker ps          # List containers
vn docker up <name>   # Start container (validates name)
vn docker down <name> # Stop container (validates name)
vn docker prune       # Clean up stopped containers
```

### Git

```bash
# Show status of all repos in ~/dev
vn git status --root ~/dev

# Sync (pull) all repos in ~/dev
vn git sync --root ~/dev
```

### Script Delegation

```bash
# Run predefined scripts (hardcoded, no dynamic loading)
vn run ubuntu22              # Run Ubuntu 22 setup
vn run win11                 # Run Windows 11 setup
vn run tools-alpine          # Run Alpine tools setup
```

### AI (Currently Stubbed)

```bash
# AI functionality is not yet available
vn ai "your prompt"
# Output: todo: AI functionality is not yet available
```

### Interactive Mode

```bash
# Launch interactive Ratatui interface
vn
```

## Configuration

Config is stored at:
- **Unix:** `~/.config/vn/config.toml`
- **Windows:** `%APPDATA%\vn\config.toml`

Default config created on first run:

```toml
[ollama]
host = "http://127.0.0.1:11434"  # (Future: for AI feature)
model = "llama3.2"                # (Future: for AI feature)
stream = true                      # (Future: for AI feature)

[sessions]
dir = "~/.local/share/vn/sessions"

[prompts]
system = "You are a local offline systems assistant for vecnode."
```

### File Permissions (Unix)

Automatically set on creation:
- Config file: `0o600` (user read/write only)
- Directories: `0o700` (user access only)

Verify:
```bash
ls -la ~/.config/vn/config.toml
# Expected: -rw------- (0o600)
```

## Deployment

### Single Binary

```bash
# Download binary from GitHub releases
curl -L -o vn https://github.com/vecnode/vecnode/releases/download/v1.0.0/vn-linux-x86_64

# Verify checksum
sha256sum -c <(grep "vn-linux-x86_64" checksums.txt)

# Install
chmod +x vn && sudo mv vn /usr/local/bin/vn
```

### Docker

```bash
docker run -v ~/.config/vn:/root/.config/vn ghcr.io/vecnode/vn:latest ai "prompt"
```

### Homebrew (macOS/Linux)

```bash
brew tap vecnode/vecnode
brew install vn
```

## Troubleshooting

### Script not found

```bash
# Run from repo root, or set VECNODE_REPO_ROOT:
export VECNODE_REPO_ROOT=/path/to/vecnode
vn run win11
```

### Docker validation errors

Ensure service name contains only alphanumerics, underscore, hyphen, period:

```bash
# ✅ Valid
vn docker up my_service
vn docker up my-service
vn docker up my.service

# ❌ Invalid
vn docker up my@service  # Error: invalid docker service name
```

### Git path traversal errors

Avoid ".." in git root paths:

```bash
# ✅ Valid
vn git status --root ~/dev

# ❌ Invalid
vn git status --root /tmp/..  # Error: path traversal detected
```

## Development

### Prerequisites

- Rust 1.70+ (see `cli/rust-toolchain.toml`)
- Cargo

### Build

```bash
cd cli
cargo build --release --locked
```

### Run Tests

```bash
cd cli
cargo test --all
```

### Run Security Audits

```bash
cd cli
cargo audit                    # Check RustSec advisories
cargo deny check advisories    # Check dependency advisories
```

### Code Formatting

```bash
cargo fmt
```

### Linting

```bash
cargo clippy --all-targets -- -D warnings
```

## Contributing

See [../CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## License

See [../LICENSE](../LICENSE)

## Support

- **Documentation:** [../docs](../docs)
- **Security:** [../SECURITY.md](../SECURITY.md)
- **Issues:** [GitHub Issues](https://github.com/vecnode/vecnode/issues)
- **Security Report:** [security@vecnode.io](mailto:security@vecnode.io)

## Config

Auto-created on first run:

- Linux: ~/.config/vn/config.toml
- Windows: %APPDATA%\\vn\\config.toml

Default example:

```toml
[ollama]
host = "http://127.0.0.1:11434"
model = "llama3.2"
stream = true

[sessions]
dir = "~/.local/share/vn/sessions"

[prompts]
system = "You are a local offline systems assistant for vecnode."
```

## Ollama Requirements

Local Ollama server should be running on the configured host.

```bash
ollama serve
ollama list
```

## Static musl Build (Linux)

```bash
cd cli
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

Binary output:

- cli/target/x86_64-unknown-linux-musl/release/vn

## Notes on Delegation

Existing scripts remain unchanged. The Rust CLI delegates where appropriate:

- scripts/ubuntu22/main.sh
- scripts/win11/main.bat
- scripts/tools-cli/alpine/main.sh
- scripts/ubuntu22/run_silverbullet.sh or scripts/win11/run_silverbullet.bat
