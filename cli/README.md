# vn CLI (Rust)

Cross-platform personal CLI for vecnode.

## Goals

- Single Rust binary command surface
- Local/offline AI via Ollama
- Script delegation to existing shell and batch workflows
- Cross-platform system and operational commands
- Ratatui interactive mode for terminal-first UX

## Install (local workspace)

```bash
cd cli
cargo build
cargo run -p vn -- --help
```

## Install (global from repo)

```bash
cd cli
cargo install --path crates/vn
```

## Core Commands

```bash
# Interactive Ratatui screen
vn

# AI
vn ai "explain this docker error"
vn ai --model llama3.2 --stream "summarize this"

# Session persistence
vn ai --session mysession "start topic"
vn ai --session mysession "continue topic"

# System
vn sys info

# Docker
vn docker ps
vn docker up silverbullet
vn docker down silverbullet
vn docker prune

# Git multi-repo helpers
vn git status --root ~/dev
vn git sync --root ~/dev

# Existing script delegation
vn run ubuntu22
vn run win11
vn run tools-alpine
```

## Config

Auto-created on first run:

- Linux: ~/.config/vn/config.toml
- macOS: ~/Library/Application Support/vn/config.toml (platform-specific config path)
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
