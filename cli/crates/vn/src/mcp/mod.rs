//! vecnode's MCP (Model Context Protocol) host: exposes vn's own host-control
//! functions as MCP tools, reachable by external MCP clients (Claude
//! Desktop/Code) via `vn mcp serve`, and by vecnode's own Ollama chat
//! in-process (see `tui/app.rs`). Two toolsets so far - apps lifecycle
//! (`apps_toolset`) and read-only docker introspection (`docker_toolset`) -
//! both implemented as `#[tool_router]` blocks on the same `AppsToolset`
//! struct (rmcp merges same-typed routers with `+`; see `AppsToolset::new`).
//! To add another: same pattern, a new `#[tool_router(router = ...)]` impl
//! block, merged in `AppsToolset::new` and dispatched from `call_by_name`.

pub mod approval;
pub mod apps_toolset;
pub mod docker_toolset;

pub use apps_toolset::AppsToolset;
