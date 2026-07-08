//! The "Apps" MCP toolset: exposes `commands::apps` (open/stop/list the
//! Dockerized vecnode apps) as MCP tools, reachable by external MCP clients
//! (Claude Desktop/Code) and by vecnode's own Ollama chat (see
//! `tui/app.rs`'s chat worker). One `AppsToolset` instance backs both the
//! standalone `vn mcp serve` server and the TUI's embedded HTTP server -
//! there is exactly one implementation of these tools. The docker toolset
//! (`mcp/docker_toolset.rs`) is a second `#[tool_router]` impl block on this
//! same struct, merged in here (see `AppsToolset::new` and `call_by_name`).

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

/// Number of tools this toolset exposes (list_apps, open_app, stop_app,
/// restart_app), plus the docker toolset merged into the same struct -
/// shown in the TUI's "MCP Server" panel.
pub const TOOL_COUNT: usize = 4 + crate::mcp::docker_toolset::TOOL_COUNT;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpenAppParams {
    /// App name - see `list_apps` for the available names.
    pub name: String,
    /// Skip opening a browser once the app is ready. Omit to use the
    /// caller's default: an external/headless MCP client (`vn mcp serve`)
    /// defaults to true (an LLM popping your browser is a surprising side
    /// effect with no one watching); vecnode's own in-TUI chat defaults to
    /// false, since the user is already present and asked for it. Pass this
    /// explicitly to override either way.
    pub no_open: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StopAppParams {
    /// App name - see `list_apps` for the available names.
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RestartAppParams {
    /// App name - see `list_apps` for the available names.
    pub name: String,
    /// Skip opening a browser once the app is ready again. Same default
    /// rules as `open_app`'s `no_open` (see there).
    pub no_open: Option<bool>,
}

#[derive(Clone)]
pub struct AppsToolset {
    loaded: LoadedConfig,
    approval: ApprovalGate,
    /// Fallback for `open_app`'s `no_open` when the caller omits it - see
    /// `AppsToolset::new`.
    default_no_open: bool,
    /// Dispatch table built by `#[tool_router]`/read by `#[tool_handler]`'s
    /// generated `ServerHandler` methods below - the dead-code lint can't see
    /// that macro-generated read, hence the allow. Merges both toolsets
    /// implemented on this struct (apps lifecycle + docker introspection).
    #[allow(dead_code)]
    tool_router: ToolRouter<AppsToolset>,
}

impl AppsToolset {
    /// `default_no_open` is the fallback used when `open_app`'s `no_open`
    /// param is omitted: pass `true` for external/headless MCP callers (`vn
    /// mcp serve`, and the TUI's own embedded server for external clients),
    /// `false` for vecnode's in-TUI Ollama chat (see module docs on
    /// `OpenAppParams::no_open`).
    pub fn new(loaded: LoadedConfig, approval: ApprovalGate, default_no_open: bool) -> Self {
        Self {
            loaded,
            approval,
            default_no_open,
            tool_router: Self::tool_router() + Self::docker_router(),
        }
    }

    /// Accessor for the docker toolset's destructive tools (`docker_stop_all`,
    /// `docker_remove_containers`, `docker_remove_images`) - they live in a
    /// different module (`docker_toolset.rs`) and so can't reach the private
    /// `approval` field directly.
    pub(crate) fn approval(&self) -> &ApprovalGate {
        &self.approval
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
        let no_open = params.no_open.unwrap_or(self.default_no_open);
        let loaded = self.loaded.clone();
        let (outcome, lines) = tokio::task::spawn_blocking(move || {
            let mut lines = Vec::new();
            let mut report = |line: &str| lines.push(line.to_string());
            let outcome = apps::open_reported(&params.name, &loaded, no_open, &mut report);
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

    #[tool(
        description = "Restart a vecnode app: stop its container (if running) and start/build it again in one call, instead of calling stop_app then open_app separately. Requires the user to approve this in the vecnode TUI; if there's no TUI attached, this is denied automatically."
    )]
    async fn restart_app(
        &self,
        Parameters(params): Parameters<RestartAppParams>,
    ) -> Result<CallToolResult, McpError> {
        let description = format!("restart_app(name=\"{}\")", params.name);
        if !self.approval.request(description).await {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "Denied: this action requires user approval in the vecnode TUI.",
            )]));
        }

        let no_open = params.no_open.unwrap_or(self.default_no_open);
        let loaded = self.loaded.clone();
        let (outcome, lines) = tokio::task::spawn_blocking(move || {
            let mut lines = Vec::new();
            let mut report = |line: &str| lines.push(line.to_string());
            let outcome = apps::stop_reported(&params.name, &loaded, &mut report)
                .and_then(|()| apps::open_reported(&params.name, &loaded, no_open, &mut report));
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
    /// an arm here too (or, for a whole new toolset, a `call_*_by_name`
    /// fallback like `call_docker_tool_by_name` below).
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
            "restart_app" => {
                let params: RestartAppParams =
                    serde_json::from_value(arguments).map_err(|err| {
                        McpError::invalid_params(
                            format!("invalid restart_app arguments: {err}"),
                            None,
                        )
                    })?;
                self.restart_app(Parameters(params)).await
            }
            other => match self.call_docker_tool_by_name(other, arguments).await {
                Some(result) => result,
                None => Err(McpError::invalid_params(
                    format!("unknown tool: {other}"),
                    None,
                )),
            },
        }
    }
}

// `router = self.tool_router.clone()`: without this, `#[tool_handler]` defaults
// to calling `Self::tool_router()` fresh, which is only the apps-lifecycle
// router - it would silently drop the docker toolset merged into the
// instance field in `AppsToolset::new`.
#[tool_handler(router = self.tool_router.clone())]
impl ServerHandler for AppsToolset {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("vecnode", env!("CARGO_PKG_VERSION")))
            .with_protocol_version(ProtocolVersion::V_2024_11_05)
            .with_instructions(
                "vecnode host controller: list/open/stop the Dockerized vecnode apps, and \
                 inspect any docker container on the host (list_containers, container_logs). \
                 stop_app requires interactive approval in the vecnode TUI.",
            )
    }
}
