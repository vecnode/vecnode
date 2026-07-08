//! The "Docker" MCP toolset: introspection into and maintenance of whatever
//! docker actually has on the host, as opposed to `apps_toolset` (which only
//! ever deals with vecnode's own [`APP_NAMES`](crate::commands::apps::APP_NAMES)).
//! Lets an MCP client (or vecnode's own Ollama chat) answer "what containers
//! exist right now", "why did that one fail", and "how much space is this
//! using" without shelling out itself, plus wrappers for `vn docker`'s
//! existing maintenance commands.
//!
//! `list_containers`, `container_logs`, `docker_check` and `disk_usage` are
//! read-only. `docker_stop_all`, `docker_remove_containers` and
//! `docker_remove_images` act host-wide (not just on vecnode's own
//! containers/images) and are hard or impossible to undo, so - like
//! `stop_app` - they go through [`ApprovalGate`](crate::mcp::approval::ApprovalGate)
//! via `AppsToolset::approval()`.
//!
//! This is a second `#[tool_router]` impl block on [`AppsToolset`] (rmcp
//! merges same-typed routers with `+`, see `AppsToolset::new`), not a
//! separate handler/server - `rmcp`'s `ServerHandler` is one struct per
//! transport, so a new toolset composes into the existing struct rather than
//! standing alongside it.

use crate::commands::apps;
use crate::mcp::AppsToolset;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{tool, tool_router, ErrorData as McpError};
use schemars::JsonSchema;
use serde::Deserialize;

/// Number of tools this toolset exposes (list_containers, container_logs,
/// docker_check, docker_stop_all, docker_remove_containers,
/// docker_remove_images, disk_usage) - added to `apps_toolset::TOOL_COUNT`
/// for the TUI's "MCP Server" panel.
pub const TOOL_COUNT: usize = 7;

/// Run a blocking, report-collecting `apps::*_reported` fn on a blocking
/// task, joining its captured lines into one string - the shared shape of
/// `docker_check`/`docker_stop_all`/`docker_remove_containers`/
/// `docker_remove_images`, which only differ in which fn they call.
async fn run_reported(
    f: impl FnOnce(&mut dyn FnMut(&str)) -> anyhow::Result<()> + Send + 'static,
) -> Result<CallToolResult, McpError> {
    let (outcome, lines) = tokio::task::spawn_blocking(move || {
        let mut lines = Vec::new();
        let mut report = |line: &str| lines.push(line.to_string());
        let outcome = f(&mut report);
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

fn default_log_lines() -> u32 {
    50
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ContainerLogsParams {
    /// Container name or ID - see `list_containers` for what exists.
    pub name: String,
    /// Number of trailing log lines to return. Defaults to 50.
    #[serde(default = "default_log_lines")]
    pub lines: u32,
}

#[tool_router(router = docker_router, vis = "pub(crate)")]
impl AppsToolset {
    #[tool(
        description = "List every docker container on this host (running or stopped), with image, state, status and published ports. Unlike list_apps, this reflects docker's actual state rather than vecnode's known app names."
    )]
    async fn list_containers(&self) -> Result<CallToolResult, McpError> {
        let containers = tokio::task::spawn_blocking(apps::docker_ps_all)
            .await
            .map_err(|err| McpError::internal_error(format!("task join error: {err}"), None))?
            .map_err(|err| McpError::internal_error(format!("{err:#}"), None))?;

        if containers.is_empty() {
            return Ok(CallToolResult::success(vec![ContentBlock::text(
                "No containers found (docker ps -a returned nothing).",
            )]));
        }

        let mut lines = vec!["ID\tNAME\tSTATE\tSTATUS\tIMAGE\tPORTS".to_string()];
        for c in containers {
            lines.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}",
                c.id, c.name, c.state, c.status, c.image, c.ports
            ));
        }
        Ok(CallToolResult::success(vec![ContentBlock::text(
            lines.join("\n"),
        )]))
    }

    #[tool(
        description = "Tail a container's logs (stdout+stderr). Use list_containers first to find the container name."
    )]
    async fn container_logs(
        &self,
        Parameters(params): Parameters<ContainerLogsParams>,
    ) -> Result<CallToolResult, McpError> {
        let text = tokio::task::spawn_blocking(move || {
            apps::docker_logs_tail(&params.name, params.lines)
        })
        .await
        .map_err(|err| McpError::internal_error(format!("task join error: {err}"), None))?
        .map_err(|err| McpError::internal_error(format!("{err:#}"), None))?;

        Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
    }

    #[tool(
        description = "Check docker's status: confirms the daemon is running, lists running containers (`docker ps`), and reports total container/image counts."
    )]
    async fn docker_check(&self) -> Result<CallToolResult, McpError> {
        run_reported(apps::docker_check_reported).await
    }

    #[tool(
        description = "Stop every running container on this host, not just vecnode's own apps. Hard to undo cleanly if you don't know what else was running, so requires the user to approve this in the vecnode TUI; if there's no TUI attached, this is denied automatically."
    )]
    async fn docker_stop_all(&self) -> Result<CallToolResult, McpError> {
        let description = "docker_stop_all()".to_string();
        if !self.approval().request(description).await {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "Denied: this action requires user approval in the vecnode TUI.",
            )]));
        }
        run_reported(apps::docker_stop_all_reported).await
    }

    #[tool(
        description = "Stop and permanently remove every container on this host, not just vecnode's own apps. Irreversible (containers are deleted, not just stopped), so requires the user to approve this in the vecnode TUI; if there's no TUI attached, this is denied automatically."
    )]
    async fn docker_remove_containers(&self) -> Result<CallToolResult, McpError> {
        let description = "docker_remove_containers()".to_string();
        if !self.approval().request(description).await {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "Denied: this action requires user approval in the vecnode TUI.",
            )]));
        }
        run_reported(apps::docker_remove_containers_reported).await
    }

    #[tool(
        description = "Permanently remove every docker image on this host, not just vecnode's own. Irreversible (rebuilt/re-pulled from scratch next time), so requires the user to approve this in the vecnode TUI; if there's no TUI attached, this is denied automatically."
    )]
    async fn docker_remove_images(&self) -> Result<CallToolResult, McpError> {
        let description = "docker_remove_images()".to_string();
        if !self.approval().request(description).await {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                "Denied: this action requires user approval in the vecnode TUI.",
            )]));
        }
        run_reported(apps::docker_remove_images_reported).await
    }

    #[tool(
        description = "Show how much disk space docker's images, containers, local volumes, and build cache are using (wraps `docker system df`)."
    )]
    async fn disk_usage(&self) -> Result<CallToolResult, McpError> {
        let text = tokio::task::spawn_blocking(apps::docker_disk_usage)
            .await
            .map_err(|err| McpError::internal_error(format!("task join error: {err}"), None))?
            .map_err(|err| McpError::internal_error(format!("{err:#}"), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
    }
}

impl AppsToolset {
    /// Dispatch for the docker toolset's tools - called from
    /// `AppsToolset::call_by_name` alongside the apps toolset's own arms, so
    /// the Ollama chat integration can reach these tools too.
    pub(super) async fn call_docker_tool_by_name(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Option<Result<CallToolResult, McpError>> {
        match name {
            "list_containers" => Some(self.list_containers().await),
            "container_logs" => {
                let params: ContainerLogsParams = match serde_json::from_value(arguments) {
                    Ok(params) => params,
                    Err(err) => {
                        return Some(Err(McpError::invalid_params(
                            format!("invalid container_logs arguments: {err}"),
                            None,
                        )))
                    }
                };
                Some(self.container_logs(Parameters(params)).await)
            }
            "docker_check" => Some(self.docker_check().await),
            "docker_stop_all" => Some(self.docker_stop_all().await),
            "docker_remove_containers" => Some(self.docker_remove_containers().await),
            "docker_remove_images" => Some(self.docker_remove_images().await),
            "disk_usage" => Some(self.disk_usage().await),
            _ => None,
        }
    }
}
