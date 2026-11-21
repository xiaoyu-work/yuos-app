//! Human Store UI adapters to OS policy, snapshots and Agent activation.
//! Package effects use the retained Root service, never provider dialogs.

use std::{io, path::Path};
use yuos_runtime::{ask_yu, BridgeError, policy};
use serde::Serialize;

pub fn user_message(err: &BridgeError) -> String {
    if err.is_denied() {
        format!("This action requires permission ({err}). Ask the Agent to explain the request.")
    } else {
        err.to_string()
    }
}

pub(crate) fn human_ui() -> Result<(), BridgeError> {
    if std::env::var("YUOS_MCP_SERVER").as_deref() == Ok("1") {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "human UI access is unavailable in MCP").into());
    }
    Ok(())
}

pub(crate) fn read_path(path: &Path) -> Result<std::path::PathBuf, BridgeError> {
    human_ui()?;
    let path = std::path::absolute(path)?;
    policy::require("fs.read", policy::Scope::path(path.to_string_lossy()))
        .map_err(io::Error::other)?;
    Ok(path)
}

pub fn read_bytes(path: &Path) -> Result<Vec<u8>, BridgeError> {
    Ok(std::fs::read(read_path(path)?)?)
}

pub fn fs_rm(path: &Path) -> Result<(), BridgeError> {
    human_ui()?;
    crate::product::query("remove_data", serde_json::json!({"path": path}))?;
    Ok(())
}

#[derive(Serialize)]
struct StoreViewContext<'a> {
    view: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    app_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
}

impl ask_yu::Context for StoreViewContext<'_> {
    const APP_ID: &'static str = "cosmic-store";
}

#[derive(Serialize)]
struct StoreSearchContext<'a> {
    mode: &'static str,
    query: &'a str,
}

impl ask_yu::Context for StoreSearchContext<'_> {
    const APP_ID: &'static str = "cosmic-store";
}

pub fn ask_yu_home() -> Result<(), ask_yu::LaunchError> {
    ask_yu::launch(&StoreViewContext { view: "home", page: None, app_id: None, name: None })
}

pub fn ask_yu_explore(page: &str) -> Result<(), ask_yu::LaunchError> {
    ask_yu::launch(&StoreViewContext { view: "explore", page: Some(page), app_id: None, name: None })
}

pub fn ask_yu_app(app_id: &str, name: &str) -> Result<(), ask_yu::LaunchError> {
    ask_yu::launch(&StoreViewContext { view: "app", page: None, app_id: Some(app_id), name: Some(name) })
}

pub fn ask_yu_search(query: &str) -> Result<(), ask_yu::LaunchError> {
    ask_yu::launch(&StoreSearchContext { mode: "search", query })
}
