//! Manifest-bound MCP runtime for Yu OS Apps.
//!
//! [`App`] reads identity, tool descriptions, and input schemas only from the
//! authoritative App manifest. Rust code binds implementation to each declared
//! name:
//!
//! ```no_run
//! use std::sync::Arc;
//!
//! use async_trait::async_trait;
//! use yuos_sdk::mcp::{App, CallContext, Tool, ToolResult};
//! use serde_json::Value;
//!
//! struct Echo;
//!
//! #[async_trait]
//! impl Tool for Echo {
//!     fn name(&self) -> &str {
//!         "echo"
//!     }
//!
//!     async fn handle(&self, args: Value, context: CallContext) -> ToolResult {
//!         if let Err(cancelled) = context.check_cancelled() {
//!             return ToolResult::error(cancelled.to_string());
//!         }
//!         match args["text"].as_str() {
//!             Some(text) => ToolResult::text(text),
//!             None => ToolResult::error("validated text argument was unavailable"),
//!         }
//!     }
//! }
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let mut app = App::from_environment()?;
//! app.bind(Arc::new(Echo))?;
//! app.serve_stdio().await?;
//! # Ok(())
//! # }
//! ```
//!
//! [`CallContext::deadline_unix_ms`] exposes the authenticated wire deadline
//! without narrowing it to a platform-specific system-time range.

mod generated;
mod manifest;
pub mod protocol;
mod server;
mod tool;
pub mod transport;

pub use crate::generated::{McpCallContext, McpPrincipal};
pub use manifest::MAX_MANIFEST_BYTES;
pub use server::{App, AppError, CALL_CONTEXT_META_KEY, ERR_SERVER_BUSY};
pub use tool::{
    CallCancelled, CallContext, CallContextError, Progress, Tool, ToolResult, ToolResultError,
};
pub use transport::{
    in_memory_pair, Frame, InMemoryTransport, StdioTransport, Transport, TransportError,
    MAX_FRAME_BYTES,
};

/// Typed `bind` helpers for the MCP tools declared in an App manifest.
///
/// Write the result to `OUT_DIR` from `build.rs`, then
/// `include!(concat!(env!("OUT_DIR"), "/tools.rs"))`. Each tool gets a module
/// with an `Args` struct and `bind(&mut app, handler)`.
pub fn rust_tool_bindings(manifest: impl AsRef<std::path::Path>) -> Result<String, AppError> {
    manifest::load(manifest.as_ref())?.rust_bindings()
}
