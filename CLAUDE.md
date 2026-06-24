# CLAUDE.md

See [AGENTS.md](AGENTS.md) for the full guide to this repository (architecture, how the
`vn` CLI / TUI / Windows tray fit together, build & lint commands, how the TUI dispatches
to subcommands and scripts, and project conventions). It is the single source of truth
for both Claude and other agent tooling.

Quick reminders:

- Build after Rust changes: `cargo build --manifest-path cli/Cargo.toml -p vn`
- Lint: `cargo clippy --manifest-path cli/Cargo.toml -p vn`
- Run the TUI: `cargo run --manifest-path cli/Cargo.toml -p vn --`
- Open-port scan: `vn net scan` (uses RustScan; `cargo install rustscan` if missing)
