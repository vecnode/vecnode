# VN CLI

This is the main repository for vecnode CLI. 

This project does not require a root launcher script to open the TUI.
The CLI is in the Rust workspace under `cli/`, and the TUI opens when `vn` runs with no subcommand.


## Clickable Launchers

If you want to click one file and open `vn` directly:

- Windows: (double-click) `run_cli.bat`
- Linux (terminal): run `./run_cli.sh`

All launchers do the same flow:

```bash
cargo build --manifest-path cli/Cargo.toml -p vn
./cli/target/debug/vn
```

Windows launcher equivalent:

```powershell
cargo build --manifest-path cli/Cargo.toml -p vn
.\cli\target\debug\vn.exe
```


## Run From Repository Root

From the repository root folder, run:

```bash
cargo run --manifest-path cli/Cargo.toml -p vn --
```

That launches the `vn` binary and opens the interface.

## Build Once, Then Run Binary

From root:

```bash
cargo build --manifest-path cli/Cargo.toml -p vn
./cli/target/debug/vn
```

On Windows (PowerShell):

```powershell
cargo build --manifest-path cli/Cargo.toml -p vn
.\cli\target\debug\vn.exe
```

## Install Globally (Optional)

From root:

```bash
cargo install --path cli/crates/vn
vn
```

