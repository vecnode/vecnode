//! vecnode's MCP (Model Context Protocol) host: exposes vn's own host-control
//! functions as MCP tools, reachable by external MCP clients (Claude
//! Desktop/Code) via `vn mcp serve`, and by vecnode's own Ollama chat
//! in-process (see `tui/app.rs`). v1 has one toolset (`apps_toolset`); to add
//! another, follow its pattern and compose it alongside `AppsToolset` where
//! the server is built (`commands/mcp.rs`, `tui/app.rs`).

pub mod approval;
pub mod apps_toolset;

pub use apps_toolset::AppsToolset;
