//! The "Apps" MCP toolset: exposes `commands::apps` (open/stop/list the
//! Dockerized vecnode apps) as MCP tools, reachable by external MCP clients
//! (Claude Desktop/Code) and by vecnode's own Ollama chat (see
//! `tui/app.rs`'s chat worker). One `AppsToolset` instance backs both the
//! standalone `vn mcp serve` server and the TUI's embedded HTTP server -
//! there is exactly one implementation of these tools.

use crate::commands::apps;
use crate::config::LoadedConfig;
use crate::mcp::approval::ApprovalGate;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;

/// Number of tools this toolset exposes (list_apps, open_app, stop_app) -
/// shown in the TUI's "MCP Server" panel.
pub const TOOL_COUNT: usize = 3;

fn default_no_open() -> bool {
    // MCP-triggered opens default to not popping a browser window: an LLM
    // (local or remote) opening your browser is a surprising side effect.
    // Pass `no_open: false` explicitly to open it anyway.
    true
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpenAppParams {
    /// App name - see `list_apps` for the available names.
    pub name: String,
    /// Skip opening a browser once the app is ready. Defaults to true for
    /// MCP-triggered opens.
    #[serde(default = "default_no_open")]
    pub no_open: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StopAppParams {
    /// App name - see `list_apps` for the available names.
    pub name: String,
}

#[derive(Clone)]
pub struct AppsToolset {
    loaded: LoadedConfig,
    approval: ApprovalGate,
    /// Dispatch table built by `#[tool_router]`/read by `#[tool_handler]`'s
    /// generated `ServerHandler` methods below - the dead-code lint can't see
    /// that macro-generated read, hence the allow.
    #[allow(dead_code)]
    tool_router: ToolRouter<AppsToolset>,
}

impl AppsToolset {
    pub fn new(loaded: LoadedConfig, approval: ApprovalGate) -> Self {
        Self {
            loaded,
            approval,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl AppsToolset {
    #[tool(
        description = "List the Dockerized vecnode apps that can be opened or stopped (e.g. silverbullet, library-portal, stirling-pdf, media-downloader, doc-processor, docs)."
    )]
    async fn list_apps(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![ContentBlock::text(
            apps::APP_NAMES.join(", "),
        )]))
    }

    #[tool(
        description = "Build/start a Dockerized vecnode app (creates and waits for the container to become ready)."
    )]
    async fn open_app(
        &self,
        Parameters(params): Parameters<OpenAppParams>,
    ) -> Result<CallToolResult, McpError> {
        let loaded = self.loaded.clone();
        let (outcome, lines) = tokio::task::spawn_blocking(move || {
            let mut lines = Vec::new();
            let mut report = |line: &str| lines.push(line.to_string());
            let outcome = apps::open_reported(&params.name, &loaded, params.no_open, &mut report);
            (outcome, lines)
        })
        .await
        .map_err(|err| McpError::internal_error(format!("task join error: {err}"), None))?;

        match outcome {
            Ok(()) => Ok(CallToolResult::success(vec![ContentBlock::text(
                lines.join("\n"),
            )])),
            Err(err) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "{err:#}\n{}",
                lines.join("\n")
            ))])),
        }
    }

    #[tool(
        description = "Stop a running vecnode app's container. Requires the user to approve this in the vecnode TUI; if there's no TUI attached, this is denied automatically."
    )]
    async fn stop_app(
        &self,
        Parameters(params): Parameters<StopAppParams>,
    ) -> Result<CallToolResult, McpError> {
        let description = format!("stop_app(name=\"{}\")", params.name);
        if !self.approval.request(description).await {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "Denied: this action requires user approval in the vecnode TUI.",
            )]));
        }

        let loaded = self.loaded.clone();
        let (outcome, lines) = tokio::task::spawn_blocking(move || {
            let mut lines = Vec::new();
            let mut report = |line: &str| lines.push(line.to_string());
            let outcome = apps::stop_reported(&params.name, &loaded, &mut report);
            (outcome, lines)
        })
        .await
        .map_err(|err| McpError::internal_error(format!("task join error: {err}"), None))?;

        match outcome {
            Ok(()) => Ok(CallToolResult::success(vec![ContentBlock::text(
                lines.join("\n"),
            )])),
            Err(err) => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "{err:#}\n{}",
                lines.join("\n")
            ))])),
        }
    }
}

impl AppsToolset {
    /// The tool definitions (name/description/JSON schema) this toolset
    /// exposes - used by the Ollama chat integration (`tui/app.rs`) to build
    /// the same tool list an external MCP client would see via `tools/list`.
    pub fn list_tools(&self) -> Vec<rmcp::model::Tool> {
        self.tool_router.list_all()
    }

    /// Call a tool by name with raw JSON arguments. Used by the Ollama chat
    /// integration, which discovers tools dynamically via `list_tools` and
    /// needs to invoke whichever one the model picked - unlike the MCP
    /// transports (stdio/HTTP), it has no `RequestContext` to route through
    /// `ToolRouter::call`, so this matches by name directly. New tools need
    /// an arm here too.
    pub async fn call_by_name(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, McpError> {
        match name {
            "list_apps" => self.list_apps().await,
            "open_app" => {
                let params: OpenAppParams = serde_json::from_value(arguments).map_err(|err| {
                    McpError::invalid_params(format!("invalid open_app arguments: {err}"), None)
                })?;
                self.open_app(Parameters(params)).await
            }
            "stop_app" => {
                let params: StopAppParams = serde_json::from_value(arguments).map_err(|err| {
                    McpError::invalid_params(format!("invalid stop_app arguments: {err}"), None)
                })?;
                self.stop_app(Parameters(params)).await
            }
            other => Err(McpError::invalid_params(
                format!("unknown tool: {other}"),
                None,
            )),
        }
    }
}

#[tool_handler]
impl ServerHandler for AppsToolset {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("vecnode", env!("CARGO_PKG_VERSION")))
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_instructions(
                "vecnode host controller: list/open/stop the Dockerized vecnode apps. \
                 stop_app requires interactive approval in the vecnode TUI.",
            )
    }
}
