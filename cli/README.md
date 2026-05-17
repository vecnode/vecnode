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

