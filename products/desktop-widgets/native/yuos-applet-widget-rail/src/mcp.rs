// SPDX-License-Identifier: GPL-3.0-only

use std::sync::Arc;

use async_trait::async_trait;
use yuos_sdk::mcp::{App, AppError, CallContext, Tool, ToolResult};
use serde_json::{Value, json};

use crate::service::Client;

struct System(Client);

#[async_trait]
impl Tool for System {
    fn name(&self) -> &str {
        "widget-rail.system"
    }

    async fn handle(&self, input: Value, context: CallContext) -> ToolResult {
        if let Err(error) = context.check_cancelled() {
            return ToolResult::error(error.to_string());
        }
        if !input.as_object().is_some_and(|args| args.is_empty()) {
            return ToolResult::error("Widget Rail system telemetry takes no arguments.");
        }
        tokio::select! {
            biased;
            _ = context.cancelled() => ToolResult::error("Widget Rail call cancelled or deadline exceeded."),
            result = self.0.system() => match result {
                Ok(summary) => ToolResult::structured(json!({"summary": summary}))
                    .unwrap_or_else(|error| ToolResult::error(error.to_string())),
                Err(error) => ToolResult::error(error),
            },
        }
    }
}

fn bind(mut app: App, client: Client) -> Result<App, AppError> {
    if app.id() != "widget-rail" {
        return Err(AppError::Manifest("Widget Rail requires the widget-rail identity".into()));
    }
    app.bind(Arc::new(System(client)))?;
    Ok(app)
}

pub(crate) fn run() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(bind(App::from_environment()?, Client::installed())?.serve_stdio())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/test/unit/mcp.rs"));
}
