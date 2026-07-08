//! Shared plumbing for report-based MCP tools (every `apps::*_reported`
//! function's shape: do work, call a `&mut dyn FnMut(&str)` per progress
//! line). Used by both `apps_toolset.rs` and `docker_toolset.rs` so neither
//! hand-rolls the spawn_blocking/collect-lines/build-`CallToolResult` dance,
//! and so both stream progress the same way to in-process callers.

use crate::mcp::approval::ApprovalGate;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::ErrorData as McpError;
use std::sync::Arc;

/// Sink for progress lines as a report-based tool call produces them.
/// Supplied only by in-process callers - currently just the TUI's chat
/// integration, via `AppsToolset::call_by_name` - that want to see output as
/// it happens instead of only the final joined result once the whole call
/// (e.g. a multi-minute docker build) completes. External MCP clients
/// (stdio/HTTP `tools/call`) never get one: MCP's tool-call protocol is
/// request/response, so they only ever see the final `CallToolResult`.
pub(crate) type LiveReporter = Arc<dyn Fn(&str) + Send + Sync>;

/// Run a blocking, report-collecting fn (the shape every `apps::*_reported`
/// function has) on a blocking task: forwards each line to `live` (if any) as
/// it's produced, and joins the captured lines into the final
/// `CallToolResult` once done - success on `Ok`, the captured lines plus the
/// error on `Err`, matching what every hand-rolled version of this used to do
/// individually.
pub(crate) async fn run_reported(
    live: Option<LiveReporter>,
    f: impl FnOnce(&mut dyn FnMut(&str)) -> anyhow::Result<()> + Send + 'static,
) -> Result<CallToolResult, McpError> {
    let (outcome, lines) = tokio::task::spawn_blocking(move || {
        let mut lines = Vec::new();
        let mut report = |line: &str| {
            lines.push(line.to_string());
            if let Some(live) = &live {
                live(line);
            }
        };
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

/// Request approval for a destructive tool call, returning the standard
/// "denied" `CallToolResult` if the user (or the fail-closed headless
/// default) doesn't approve - shared so every gated tool
/// (`stop_app`/`restart_app`/`docker_stop_all`/`docker_remove_containers`/
/// `docker_remove_images`) produces identical denial text instead of each
/// repeating it.
pub(crate) async fn require_approval(
    approval: &ApprovalGate,
    description: String,
) -> Option<CallToolResult> {
    if approval.request(description).await {
        None
    } else {
        Some(CallToolResult::error(vec![ContentBlock::text(
            "Denied: this action requires user approval in the vecnode TUI.",
        )]))
    }
}
