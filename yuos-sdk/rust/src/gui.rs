//! Desktop GUI bootstrap for Yu OS apps (Rust edition).
//!
//! This is the Rust counterpart to [`crate::ai`] (gated model access)
//! and Python's `yuos_sdk.gui`. It deliberately does **not** wrap a
//! UI toolkit: a Yu OS desktop app draws its own window in whatever
//! toolkit it likes ("World A"). This module only surfaces the small
//! amount of kernel context an app receives once it has been
//! kernel-spawned as a GUI, plus the one privileged action a GUI
//! commonly wants — summoning the system agent overlay.
//!
//! # How a GUI app is launched
//!
//! When an app declares a `desktop` block in `app.json`, `yu app
//! install` writes a launcher whose `Exec` is `yu app <id> --gui`.
//! Activating it routes the launch back through the kernel, which spawns
//! the app with `YUOS_APP_GUI=1` and `YUOS_APP_ID` set, so identity,
//! audit, and consent apply to the GUI exactly as to the headless path.
//!
//! # Usage
//!
//! ```no_run
//! use yuos_sdk::{app::App, gui};
//!
//! fn main() -> Result<(), yuos_sdk::app::AppError> {
//!     App::new()
//!         .desktop(|| {
//!             let ctx = gui::Context::from_env()?;
//!             // Draw your own window using ctx.app_id / ctx.files,
//!             // and call ctx.open_agent_overlay(None) to summon "Ask Yu OS".
//!             Ok(())
//!         })
//!         .run()
//! }
//! ```

#[cfg(target_os = "linux")]
use std::io::{BufRead, BufReader, Read, Write};
#[cfg(target_os = "linux")]
use std::os::linux::net::SocketAddrExt;
#[cfg(target_os = "linux")]
use std::os::unix::net::{SocketAddr, UnixStream};
#[cfg(target_os = "linux")]
use std::process::{Command, Stdio};
#[cfg(target_os = "linux")]
use std::sync::mpsc;
#[cfg(target_os = "linux")]
use std::time::Duration;

#[cfg(target_os = "linux")]
const ASK_YU_LAUNCHER: &str = "/usr/local/bin/yuos-ask-launcher";
#[cfg(target_os = "linux")]
const ASK_YU_PROTOCOL: u32 = 1;
#[cfg(target_os = "linux")]
const ASK_YU_REQUEST_LIMIT: usize = 32 * 1024;
#[cfg(target_os = "linux")]
const ASK_YU_TIMEOUT: Duration = Duration::from_secs(5);

/// Returns `true` when the current process was spawned as a desktop GUI.
///
/// The bridge sets `YUOS_APP_GUI=1` for the authenticated desktop launch.
pub fn is_gui_launch() -> bool {
    std::env::var("YUOS_APP_GUI").as_deref() == Ok("1")
}

/// Package-local, read-only presentation resources supplied by the Host.
///
/// This path is context, not a filesystem permission or proof of authority.
pub fn resource_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("YUOS_APP_RESOURCE_DIR")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
}

/// Authenticated owner's home for UI path choices, distinct from App state.
///
/// Access still requires the App's exact filesystem capabilities and sandbox.
pub fn owner_home() -> Option<std::path::PathBuf> {
    std::env::var_os("YUOS_OWNER_HOME")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
}

/// Propose a native FUSE metadata inode for protected owner read review.
/// A pending response is not selected content or permission to read.
pub fn request_file_read(node: u64) -> Result<crate::generated::GuiFileRead, crate::BridgeError> {
    let request = serde_json::json!({ "node": node });
    crate::generated::validate_gui_file_read_request(&request)
        .map_err(|error| selected_read_decode("request", error))?;
    selected_read_call("request", request)
}

/// Read this window's Root-owned selection status for the same proposed inode.
pub fn file_read_status(
    selection_id: &str,
    node: u64,
) -> Result<crate::generated::GuiFileRead, crate::BridgeError> {
    let request = serde_json::json!({ "selection_id": selection_id, "node": node });
    crate::generated::validate_gui_file_read_status_request(&request)
        .map_err(|error| selected_read_decode("status", error))?;
    selected_read_call("status", request)
}

/// Propose this window's native inode for an independent protected write decision.
/// A pending response never grants native write or content read.
pub fn request_file_write(node: u64) -> Result<crate::generated::GuiFileWrite, crate::BridgeError> {
    let request = serde_json::json!({ "node": node });
    crate::generated::validate_gui_file_write_request(&request)
        .map_err(|error| selected_file_decode("__gui-file-write", "request", error))?;
    selected_write_call("request", request)
}

/// Query the same window and target's independent write selection.
pub fn file_write_status(
    selection_id: &str,
    node: u64,
) -> Result<crate::generated::GuiFileWrite, crate::BridgeError> {
    let request = serde_json::json!({ "selection_id": selection_id, "node": node });
    crate::generated::validate_gui_file_write_status_request(&request)
        .map_err(|error| selected_file_decode("__gui-file-write", "status", error))?;
    selected_write_call("status", request)
}

/// Reserve one private same-parent scratch name for the selected target only.
/// Creation, writing, fsync and rename still use normal native filesystem IO.
pub fn reserve_file_save(
    selection_id: &str,
    node: u64,
) -> Result<crate::generated::GuiFileSave, crate::BridgeError> {
    let request = serde_json::json!({ "selection_id": selection_id, "node": node });
    crate::generated::validate_gui_file_save_reservation_request(&request)
        .map_err(|error| selected_file_decode("__gui-file-write", "reserve", error))?;
    let mut response = selected_file_call("__gui-file-write", "reserve", request)?;
    crate::generated::validate_gui_file_save(&response)
        .map_err(|error| selected_file_decode("__gui-file-write", "reserve", error))?;
    crate::generated::normalize_gui_file_save_integers(&mut response);
    serde_json::from_value(response)
        .map_err(|error| selected_file_decode("__gui-file-write", "reserve", error))
}

fn selected_read_decode(verb: &str, error: impl std::fmt::Display) -> crate::BridgeError {
    selected_file_decode("__gui-file-read", verb, error)
}

/// Ask Root to review one absent literal child of this metadata directory.
/// Selected/reserved is not saved and never grants owner-content read.
pub fn request_file_create(parent_node: u64, name: &str) -> Result<crate::generated::GuiFileCreate, crate::BridgeError> {
    let request = serde_json::json!({"parent_node":parent_node,"name":name});
    crate::generated::validate_gui_file_create_request(&request)
        .map_err(|error| selected_file_decode("__gui-file-create", "request", error))?;
    selected_create_call("request", request)
}

pub fn file_create_status(selection_id: &str, parent_node: u64) -> Result<crate::generated::GuiFileCreate, crate::BridgeError> {
    let request = serde_json::json!({"selection_id":selection_id,"parent_node":parent_node});
    crate::generated::validate_gui_file_create_status_request(&request)
        .map_err(|error| selected_file_decode("__gui-file-create", "status", error))?;
    selected_create_call("status", request)
}

/// Reserve private scratch for one atomic NOREPLACE creation, not overwrite.
pub fn reserve_file_create(selection_id: &str, parent_node: u64) -> Result<crate::generated::GuiFileCreateSlot, crate::BridgeError> {
    let request = serde_json::json!({"selection_id":selection_id,"parent_node":parent_node});
    crate::generated::validate_gui_file_create_reservation_request(&request)
        .map_err(|error| selected_file_decode("__gui-file-create", "reserve", error))?;
    let mut response = selected_file_call("__gui-file-create", "reserve", request)?;
    crate::generated::validate_gui_file_create_slot(&response)
        .map_err(|error| selected_file_decode("__gui-file-create", "reserve", error))?;
    crate::generated::normalize_gui_file_create_slot_integers(&mut response);
    serde_json::from_value(response)
        .map_err(|error| selected_file_decode("__gui-file-create", "reserve", error))
}

fn selected_create_call(verb: &str, request: serde_json::Value) -> Result<crate::generated::GuiFileCreate, crate::BridgeError> {
    let mut response = selected_file_call("__gui-file-create", verb, request)?;
    crate::generated::validate_gui_file_create(&response)
        .map_err(|error| selected_file_decode("__gui-file-create", verb, error))?;
    crate::generated::normalize_gui_file_create_integers(&mut response);
    serde_json::from_value(response)
        .map_err(|error| selected_file_decode("__gui-file-create", verb, error))
}

fn selected_file_decode(bridge: &str, verb: &str, error: impl std::fmt::Display) -> crate::BridgeError {
    crate::BridgeError::Decode {
        app: bridge.into(), verb: verb.into(), message: error.to_string(),
    }
}

fn selected_read_call(
    verb: &str,
    request: serde_json::Value,
) -> Result<crate::generated::GuiFileRead, crate::BridgeError> {
    let response = selected_file_call("__gui-file-read", verb, request)?;
    selected_read_response(verb, response)
}

fn selected_file_call(
    bridge: &str, verb: &str, request: serde_json::Value,
) -> Result<serde_json::Value, crate::BridgeError> {
    crate::yu_call_json_with_stdin(bridge, verb, [bridge, verb, "--request-stdin"], request.to_string().as_bytes())
}

fn selected_write_call(
    verb: &str, request: serde_json::Value,
) -> Result<crate::generated::GuiFileWrite, crate::BridgeError> {
    let mut response = selected_file_call("__gui-file-write", verb, request)?;
    crate::generated::validate_gui_file_write(&response)
        .map_err(|error| selected_file_decode("__gui-file-write", verb, error))?;
    crate::generated::normalize_gui_file_write_integers(&mut response);
    serde_json::from_value(response)
        .map_err(|error| selected_file_decode("__gui-file-write", verb, error))
}

fn selected_read_response(
    verb: &str,
    mut response: serde_json::Value,
) -> Result<crate::generated::GuiFileRead, crate::BridgeError> {
    crate::generated::validate_gui_file_read(&response)
        .map_err(|error| selected_read_decode(verb, error))?;
    crate::generated::normalize_gui_file_read_integers(&mut response);
    serde_json::from_value(response).map_err(|error| selected_read_decode(verb, error))
}

fn hex_nibble(byte: u8) -> Option<u8> {
    (byte as char).to_digit(16).map(|d| d as u8)
}

fn percent_decode_path(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if let Some((hi, lo)) = (bytes[i] == b'%' && i + 2 < bytes.len())
            .then(|| hex_nibble(bytes[i + 1]).zip(hex_nibble(bytes[i + 2])))
            .flatten()
        {
            out.push((hi << 4) | lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn next_picker_request_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let high = nanos ^ ((std::process::id() as u64) << 32);
    format!("{high:016x}{seq:016x}")
}

/// The kernel context handed to a desktop app at launch.
fn run_picker(
    mode: &str,
    title: &str,
    dev_env: &str,
    name: Option<&str>,
) -> Result<Option<String>, String> {
    if let (None, Ok(dev_file)) = (std::env::var_os("YUOS_SESSION"), std::env::var(dev_env)) {
        return Ok((!dev_file.is_empty()).then_some(dev_file));
    }
    let request_id = next_picker_request_id();
    let mut payload = serde_json::json!({
        "request_id": request_id,
        "title": title,
        "modal": true,
    });
    if mode != "file" {
        payload["mode"] = serde_json::json!(mode);
    }
    if let Some(n) = name {
        payload["name"] = serde_json::json!(n);
    }
    selected_file_call("__gui-file-picker", "open", payload).map_err(|e| e.to_string())?;
    loop {
        let res = selected_file_call(
            "__gui-file-picker",
            "status",
            serde_json::json!({"request_id": request_id}),
        )
        .map_err(|e| e.to_string())?;
        match res.get("status").and_then(serde_json::Value::as_str) {
            Some("pending") => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Some("cleanup_pending") => {
                return Err("OS file picker cleanup is unconfirmed; do not replay".into());
            }
            Some("selected") => {
                let uri = res.get("uri").and_then(serde_json::Value::as_str).unwrap_or("");
                let path = match uri.strip_prefix("file://") {
                    Some(raw) => percent_decode_path(raw),
                    None => uri.to_string(),
                };
                return Ok(Some(path));
            }
            Some("cancelled") => return Ok(None),
            _ => {
                let err = res
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("file picker failed");
                return Err(err.to_string());
            }
        }
    }
}

/// Open the system file picker and return the selected file path, or `None` if cancelled.
pub fn open_file(title: &str) -> Result<Option<String>, String> {
    let title = if title.is_empty() { "Open file" } else { title };
    run_picker("file", title, "YUOS_DEV_OPEN_FILE", None)
}

/// Open the system Save dialog and return the chosen path, or `None` if cancelled.
pub fn save_file(default_name: &str, title: &str) -> Result<Option<String>, String> {
    if default_name.trim().is_empty() {
        return Err("save_file: default_name must be non-empty".into());
    }
    let title = if title.is_empty() { "Save file" } else { title };
    run_picker("save", title, "YUOS_DEV_SAVE_FILE", Some(default_name))
}

/// Open the system folder picker and return the selected directory path, or `None` if cancelled.
pub fn select_folder(title: &str) -> Result<Option<String>, String> {
    let title = if title.is_empty() { "Select folder" } else { title };
    run_picker("folder", title, "YUOS_DEV_SELECT_FOLDER", None)
}

#[derive(Debug, Clone)]
pub struct Context {
    /// The kernel-assigned app identity (`YUOS_APP_ID`).
    pub app_id: String,
    /// File paths the launcher passed (`%F`), decoded from
    /// `YUOS_ARGS_JSON`. Empty when launched without file arguments.
    pub files: Vec<String>,
}

impl Context {
    /// Build the context from the environment the kernel set up.
    pub fn from_env() -> Result<Self, String> {
        let app_id = std::env::var("YUOS_APP_ID")
            .map_err(|_| "YUOS_APP_ID is required for a GUI launch".to_string())?;
        if app_id.is_empty() {
            return Err("YUOS_APP_ID is required for a GUI launch".to_string());
        }
        let files = match std::env::var("YUOS_ARGS_JSON") {
            Ok(raw) => serde_json::from_str::<Vec<String>>(&raw)
                .map_err(|error| format!("YUOS_ARGS_JSON must be an array of strings: {error}"))?,
            Err(std::env::VarError::NotPresent) => Vec::new(),
            Err(error) => return Err(format!("read YUOS_ARGS_JSON: {error}")),
        };
        Ok(Self { app_id, files })
    }

    /// Summon the system "Ask Yu OS" agent overlay.
    ///
    /// This uses the fixed packaged Ask Yu OS launcher and its versioned
    /// authenticated Unix-socket protocol. Pass `hint` to ground the agent's first response in
    /// the app's current state (e.g. the open document) without
    /// polluting the visible chat transcript. The overlay is detached —
    /// it outlives this call and is not tied to the app's stdio.
    ///
    /// Returns the spawn error if the overlay binary is missing (e.g. a
    /// headless box with no desktop shell).
    pub fn open_agent_overlay(&self, hint: Option<&str>) -> std::io::Result<()> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = hint;
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Ask Yu OS private activation requires Linux",
            ));
        }
        #[cfg(target_os = "linux")]
        {
            validate_launcher()?;
            let mut child = Command::new(ASK_YU_LAUNCHER)
                .args(["--protocol", &ASK_YU_PROTOCOL.to_string()])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?;
            let Some(stdout) = child.stdout.take() else {
                return Err(stop_launcher(
                    &mut child,
                    std::io::Error::other("Ask Yu OS launcher readiness channel unavailable"),
                ));
            };
            let (sender, receiver) = mpsc::sync_channel(1);
            std::thread::spawn(move || {
                let mut ready = String::new();
                let result = BufReader::new(stdout)
                    .take(257)
                    .read_line(&mut ready)
                    .and_then(|_| {
                        if ready.len() > 256 || !ready.ends_with('\n') {
                            Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "invalid Ask Yu OS socket announcement",
                            ))
                        } else {
                            Ok(ready)
                        }
                    });
                let _ = sender.send(result);
            });
            let announcement = match receiver.recv_timeout(ASK_YU_TIMEOUT) {
                Ok(Ok(announcement)) => announcement,
                Ok(Err(error)) => return Err(stop_launcher(&mut child, error)),
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Ask Yu OS launcher readiness timed out",
                    ));
                }
            };
            let endpoint = announcement
                .strip_prefix(&format!("SOCKET {ASK_YU_PROTOCOL} @"))
                .and_then(|value| value.strip_suffix('\n'))
                .filter(|value| !value.is_empty())
                .ok_or_else(|| std::io::Error::other("invalid Ask Yu OS socket announcement"));
            let endpoint = match endpoint {
                Ok(endpoint) => endpoint,
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(error);
                }
            };
            let address = match SocketAddr::from_abstract_name(endpoint.as_bytes()) {
                Ok(address) => address,
                Err(error) => return Err(stop_launcher(&mut child, error)),
            };
            let mut socket = match UnixStream::connect_addr(&address) {
                Ok(socket) => socket,
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(error);
                }
            };
            if let Err(error) = socket.set_read_timeout(Some(ASK_YU_TIMEOUT)) {
                return Err(stop_launcher(&mut child, error));
            }
            if let Err(error) = socket.set_write_timeout(Some(ASK_YU_TIMEOUT)) {
                return Err(stop_launcher(&mut child, error));
            }
            let mut ready = [0_u8; 8];
            if let Err(error) = socket.read_exact(&mut ready) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
            if ready != *b"READY 1\n" {
                let _ = child.kill();
                let _ = child.wait();
                return Err(std::io::Error::other("invalid Ask Yu OS socket handshake"));
            }
            let request = serde_json::json!({
                "protocol": ASK_YU_PROTOCOL,
                "app": self.app_id,
                "hint": hint,
            });
            let payload = match serde_json::to_vec(&request) {
                Ok(payload) => payload,
                Err(error) => {
                    return Err(stop_launcher(
                        &mut child,
                        std::io::Error::new(std::io::ErrorKind::InvalidData, error),
                    ));
                }
            };
            if payload.len() > ASK_YU_REQUEST_LIMIT {
                let _ = child.kill();
                let _ = child.wait();
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Ask Yu OS request exceeds the protocol limit",
                ));
            }
            if let Err(error) = socket
                .write_all(&(payload.len() as u32).to_be_bytes())
                .and_then(|_| socket.write_all(&payload))
                .and_then(|_| socket.flush())
                .and_then(|_| socket.shutdown(std::net::Shutdown::Write))
            {
                return Err(stop_launcher(&mut child, error));
            }
            let mut accepted = [0_u8; 11];
            if let Err(error) = socket.read_exact(&mut accepted) {
                return Err(stop_launcher(&mut child, error));
            }
            if accepted != *b"ACCEPTED 1\n" {
                let _ = child.kill();
                let _ = child.wait();
                return Err(std::io::Error::other(
                    "invalid Ask Yu OS acceptance response",
                ));
            }
            drop(socket);
            let child = std::sync::Arc::new(std::sync::Mutex::new(Some(child)));
            let reaper_child = std::sync::Arc::clone(&child);
            if let Err(error) = std::thread::Builder::new()
                .name("ask-yu-sdk-reaper".into())
                .spawn(move || {
                    if let Some(mut child) = reaper_child
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .take()
                    {
                        let _ = child.wait();
                    }
                })
            {
                if let Some(mut child) = child
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .take()
                {
                    return Err(stop_launcher(&mut child, error));
                }
                return Err(error);
            }
            Ok(())
        }
    }
}

#[cfg(target_os = "linux")]
fn stop_launcher(child: &mut std::process::Child, error: std::io::Error) -> std::io::Error {
    let _ = child.kill();
    let _ = child.wait();
    error
}

#[cfg(target_os = "linux")]
fn validate_launcher() -> std::io::Result<()> {
    use std::os::unix::fs::MetadataExt;

    for path in ["/usr", "/usr/local", "/usr/local/bin"] {
        let metadata = std::fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || metadata.uid() != 0
            || metadata.mode() & 0o022 != 0
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "untrusted Ask Yu OS launcher parent",
            ));
        }
    }
    let metadata = std::fs::symlink_metadata(ASK_YU_LAUNCHER)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.uid() != 0
        || metadata.mode() & 0o111 == 0
        || metadata.mode() & 0o022 != 0
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "untrusted Ask Yu OS launcher",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/test/unit/gui.rs"));
}
