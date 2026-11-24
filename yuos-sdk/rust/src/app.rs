//! Command-line and desktop functions for a Yu OS App.
//!
//! ```no_run
//! use yuos_sdk::app::App;
//!
//! fn main() -> Result<(), yuos_sdk::app::AppError> {
//!     App::new()
//!         // yu app <id> summarize "text"
//!         .command("summarize", |args| Ok(serde_json::json!({"summary": args[0]})))
//!         // App launcher or yu app <id> --gui
//!         .desktop(|| Ok(()))
//!         .run()
//! }
//! ```
//!
//! Declare the command under `operations` in `app.json`.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

/// Return the App's private data directory, creating it if needed.
pub fn data_dir() -> std::io::Result<PathBuf> {
    let path = match std::env::var("YUOS_DATA_DIR") {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => {
            let base = std::env::var("XDG_DATA_HOME")
                .ok()
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
                    PathBuf::from(home).join(".local").join("share")
                });
            let app_id = std::env::var("YUOS_APP_ID")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "dev".into());
            base.join("yuos-dev").join("apps").join(app_id)
        }
    };
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// Post a system notification from this App (`ui.notify`).
pub fn notify(summary: &str, body: &str) -> Result<serde_json::Value, crate::BridgeError> {
    if summary.trim().is_empty() {
        return Err(crate::BridgeError::Decode {
            app: "__notifications".into(),
            verb: "request".into(),
            message: "notify: summary must be non-empty".into(),
        });
    }
    if std::env::var_os("YUOS_SESSION").is_none() {
        return Ok(serde_json::json!({ "status": "dev", "summary": summary, "body": body }));
    }
    let payload = serde_json::to_vec(&serde_json::json!({
        "action": "post",
        "summary": summary,
        "body": body,
        "app_name": std::env::var("YUOS_APP_ID").unwrap_or_default(),
        "icon": "",
        "expire_ms": 0,
        "transient": false,
    }))
    .map_err(|e| crate::BridgeError::Decode {
        app: "__notifications".into(),
        verb: "request".into(),
        message: e.to_string(),
    })?;
    crate::yu_call_json_stdin(
        "__notifications",
        "request",
        [
            "__notifications",
            "request",
            "--request-stdin",
            "--deadline",
            "9999999999999",
        ],
        &payload,
        false,
    )
}

/// Read text from the system clipboard (`clipboard.read`).
pub fn clipboard_read() -> Result<String, crate::BridgeError> {
    if std::env::var_os("YUOS_SESSION").is_none() {
        return Ok(std::env::var("YUOS_DEV_CLIPBOARD").unwrap_or_default());
    }
    let res = crate::yu_call_json("__clipboard", "read", ["__clipboard", "read"])?;
    Ok(res
        .get("text")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string())
}

/// Write text to the system clipboard (`clipboard.write`).
pub fn clipboard_write(text: &str) -> Result<(), crate::BridgeError> {
    if std::env::var_os("YUOS_SESSION").is_none() {
        return Ok(());
    }
    let scratch = data_dir()
        .map_err(crate::BridgeError::BinaryNotFound)?
        .join(".clipboard-tmp");
    std::fs::write(&scratch, text).map_err(crate::BridgeError::BinaryNotFound)?;
    let path_str = scratch.to_string_lossy().to_string();
    let res = crate::yu_call_json(
        "__clipboard",
        "write",
        ["__clipboard", "write", "--source", &path_str, "--confirm"],
    );
    let _ = std::fs::remove_file(&scratch);
    res.map(|_| ())
}

/// Load a secret from the Yu OS credential store (`secret.read`).
pub fn secret(name: &str, namespace: &str) -> Result<String, crate::BridgeError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(crate::BridgeError::Decode {
            app: "credential".into(),
            verb: "load".into(),
            message: "secret: name must be non-empty".into(),
        });
    }
    let namespace = if namespace.trim().is_empty() {
        "default"
    } else {
        namespace.trim()
    };
    let env_key = format!(
        "YUOS_DEV_SECRET_{}",
        name.chars()
            .map(|c| if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            })
            .collect::<String>()
    );
    if let Ok(v) = std::env::var(&env_key).or_else(|_| std::env::var("YUOS_DEV_SECRET")) {
        return Ok(v);
    }
    if std::env::var("YUOS_DEV_MODE").as_deref() == Ok("1") {
        return Ok(String::new());
    }
    let res = crate::yu_call_json(
        "credential",
        "load",
        ["credential", "load", name, "--namespace", namespace],
    )?;
    res.get("value")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| crate::BridgeError::Decode {
            app: "credential".into(),
            verb: "load".into(),
            message: format!("credential {namespace}/{name} has no value"),
        })
}

/// Dynamic scope passed to [`check_permission`] or [`require_permission`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionScope {
    Path(String),
    Host(String),
    Name(String),
    SelfRef(String),
    Wild,
    Unscoped,
}

impl PermissionScope {
    pub fn path(p: impl Into<String>) -> Self {
        Self::Path(p.into())
    }
    pub fn host(h: impl Into<String>) -> Self {
        Self::Host(h.into())
    }
    pub fn name(n: impl Into<String>) -> Self {
        Self::Name(n.into())
    }
    pub fn self_ref(s: impl Into<String>) -> Self {
        Self::SelfRef(s.into())
    }
}

/// Check whether the current session may exercise `verb` on `scope`.
pub fn check_permission(
    verb: &str,
    scope: PermissionScope,
) -> Result<serde_json::Value, crate::BridgeError> {
    let verb = verb.trim();
    if verb.is_empty() {
        return Err(crate::BridgeError::Decode {
            app: "__policy".into(),
            verb: "check".into(),
            message: "check_permission: verb must be non-empty".into(),
        });
    }
    if std::env::var("YUOS_DEV_MODE").as_deref() == Ok("1") {
        return Ok(serde_json::json!({
            "decision": "allow",
            "verb": verb,
            "granted": true,
        }));
    }
    let mut argv = vec!["__policy".to_owned(), "check".to_owned(), verb.to_owned()];
    match scope {
        PermissionScope::Path(p) => argv.extend(["--path".into(), p]),
        PermissionScope::Host(h) => argv.extend(["--host".into(), h]),
        PermissionScope::Name(n) => argv.extend(["--name".into(), n]),
        PermissionScope::SelfRef(s) => argv.extend(["--self".into(), s]),
        PermissionScope::Wild => argv.push("--wild".into()),
        PermissionScope::Unscoped => {}
    }
    let res = crate::yu_call_json("__policy", verb, argv)?;
    if res.get("decision").is_none() {
        return Err(crate::BridgeError::Decode {
            app: "__policy".into(),
            verb: verb.into(),
            message: "policy check response omitted `decision`".into(),
        });
    }
    Ok(res)
}

/// Require that the current session may exercise `verb` on `scope`.
pub fn require_permission(verb: &str, scope: PermissionScope) -> Result<(), crate::BridgeError> {
    let decision = check_permission(verb, scope)?;
    if decision.get("decision").and_then(serde_json::Value::as_str) == Some("allow") {
        Ok(())
    } else {
        let message = decision
            .get("summary")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("permission denied")
            .to_owned();
        Err(crate::BridgeError::AppError {
            app: "__policy".into(),
            verb: verb.into(),
            code: "PERMISSION_DENIED".into(),
            message,
        })
    }
}

/// Launch a desktop application by freedesktop AppID (`desktop.launch`).
pub fn launch_app(app_id: &str, uri: Option<&str>) -> Result<serde_json::Value, crate::BridgeError> {
    let app_id = app_id.trim();
    if app_id.is_empty() {
        return Err(crate::BridgeError::Decode {
            app: "desktop".into(),
            verb: "launch".into(),
            message: "launch_app: app_id must be non-empty".into(),
        });
    }
    let mut argv = vec![
        "__desktop".to_owned(),
        "launch".to_owned(),
        "--app-id".to_owned(),
        app_id.to_owned(),
    ];
    if let Some(u) = uri {
        let u = u.trim();
        if u.is_empty() || u.contains('\0') {
            return Err(crate::BridgeError::Decode {
                app: "desktop".into(),
                verb: "launch".into(),
                message: "launch_app: uri must be non-empty".into(),
            });
        }
        argv.extend(["--uri".to_owned(), u.to_owned()]);
    }
    if std::env::var_os("YUOS_SESSION").is_none() {
        return Ok(serde_json::json!({
            "launched": true,
            "app_id": app_id,
            "uri": uri,
            "status": "dev",
        }));
    }
    crate::yu_call_json("desktop", "launch", argv)
}

/// Open an http(s) or mailto URL in the desktop browser (`desktop.launch`).
pub fn open_url(url: &str) -> Result<serde_json::Value, crate::BridgeError> {
    let u = url.trim();
    if !u.starts_with("http://") && !u.starts_with("https://") && !u.starts_with("mailto:") {
        return Err(crate::BridgeError::Decode {
            app: "desktop".into(),
            verb: "launch".into(),
            message: "open_url: url must start with http://, https://, or mailto:".into(),
        });
    }
    launch_app("com.yuos.Browser", Some(u))
}

/// Record a structured App log entry (`data.log.write`) to `data_dir()/logs/app.jsonl` and stderr.
pub fn log(level: &str, message: &str) -> Result<serde_json::Value, crate::BridgeError> {
    if message.trim().is_empty() {
        return Err(crate::BridgeError::Decode {
            app: "log".into(),
            verb: "write".into(),
            message: "log: message must be non-empty".into(),
        });
    }
    if !matches!(level, "debug" | "info" | "warn" | "error") {
        return Err(crate::BridgeError::Decode {
            app: "log".into(),
            verb: "write".into(),
            message: "log: level must be debug, info, warn, or error".into(),
        });
    }
    if std::env::var_os("YUOS_SESSION").is_some() {
        require_permission("data.log.write", PermissionScope::Wild)?;
    }
    let dir = data_dir()
        .map_err(crate::BridgeError::BinaryNotFound)?
        .join("logs");
    std::fs::create_dir_all(&dir).map_err(crate::BridgeError::BinaryNotFound)?;
    let app_id = std::env::var("YUOS_APP_ID")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dev".into());
    let entry = serde_json::json!({
        "app": app_id,
        "level": level,
        "message": message,
    });
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("app.jsonl"))
        .map_err(crate::BridgeError::BinaryNotFound)?;
    let _ = writeln!(file, "{entry}");
    eprintln!("[{}] {message}", level.to_ascii_uppercase());
    Ok(entry)
}

/// Error returned by an App function.
pub type FunctionError = Box<dyn std::error::Error + Send + Sync>;

type CommandFn = Box<dyn FnOnce(Vec<String>) -> Result<serde_json::Value, FunctionError>>;
type DesktopFn = Box<dyn FnOnce() -> Result<(), FunctionError>>;

/// The App's command-line and desktop functions.
#[derive(Default)]
pub struct App {
    commands: BTreeMap<String, CommandFn>,
    desktop: Option<DesktopFn>,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("the App has no function for command `{0}`")]
    NoCommand(String),
    #[error("the App has no desktop function")]
    NoDesktop,
    #[error("YUOS_ARGS_JSON must be an array of strings: {0}")]
    Args(String),
    #[error(transparent)]
    Function(FunctionError),
    #[error("write the command result: {0}")]
    Output(std::io::Error),
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run `function` for `yu app <id> <name>`. Its value is printed as JSON.
    pub fn command<F>(mut self, name: &str, function: F) -> Self
    where
        F: FnOnce(Vec<String>) -> Result<serde_json::Value, FunctionError> + 'static,
    {
        assert!(!name.is_empty(), "command name must be nonempty");
        let previous = self.commands.insert(name.to_owned(), Box::new(function));
        assert!(previous.is_none(), "command `{name}` already has a function");
        self
    }

    /// Run `function` to open the App's desktop window.
    pub fn desktop<F>(mut self, function: F) -> Self
    where
        F: FnOnce() -> Result<(), FunctionError> + 'static,
    {
        let previous = self.desktop.replace(Box::new(function));
        assert!(previous.is_none(), "the desktop window already has a function");
        self
    }

    /// Call the function Yu OS started this process for.
    pub fn run(mut self) -> Result<(), AppError> {
        let (mut args, has_env_args) = match std::env::var("YUOS_ARGS_JSON") {
            Ok(raw) if !raw.is_empty() => (
                serde_json::from_str(&raw).map_err(|error| AppError::Args(error.to_string()))?,
                true,
            ),
            Ok(_) | Err(std::env::VarError::NotPresent) => (Vec::new(), false),
            Err(error) => return Err(AppError::Args(error.to_string())),
        };
        if crate::gui::is_gui_launch() {
            return self.run_desktop();
        }
        let mut name = std::env::var("YUOS_COMMAND").unwrap_or_default();
        if name.is_empty() {
            let cli_args: Vec<String> = std::env::args().skip(1).collect();
            if cli_args.first().map(String::as_str) == Some("--gui") {
                return self.run_desktop();
            }
            if let Some((first, rest)) = cli_args.split_first() {
                name = first.clone();
                if !has_env_args {
                    args = rest.to_vec();
                }
            }
        }
        let has_desktop = self.desktop.is_some();
        let Some(function) = self.commands.remove(&name) else {
            if has_desktop {
                let fallback = serde_json::json!({
                    "available": false,
                    "surface": "cli",
                    "command": name,
                    "message": "This feature is only available in the desktop app.",
                });
                return writeln!(std::io::stdout(), "{fallback}").map_err(AppError::Output);
            }
            return Err(AppError::NoCommand(name));
        };
        let result = function(args).map_err(AppError::Function)?;
        if result.is_null() {
            return Ok(());
        }
        writeln!(std::io::stdout(), "{result}").map_err(AppError::Output)
    }

    fn run_desktop(self) -> Result<(), AppError> {
        if let Some(function) = self.desktop {
            return function().map_err(AppError::Function);
        }
        if !self.commands.is_empty() {
            let fallback = serde_json::json!({
                "available": false,
                "surface": "desktop",
                "message": "This feature is only available on the command line.",
            });
            return writeln!(std::io::stdout(), "{fallback}").map_err(AppError::Output);
        }
        Err(AppError::NoDesktop)
    }
}
