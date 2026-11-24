//! Call the user's configured Yu OS models from an authorized App.
//!
//! ```no_run
//! use yuos_sdk::ai;
//!
//! fn summarise(body: &str) -> Result<String, ai::AiError> {
//!     let r = ai::chat(body, ai::ChatOpts::default())?;
//!     Ok(r.output.text)
//! }
//! ```

use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};

use crate::{yu_call_json_structured, BridgeError, StructuredCosError};
pub use crate::generated::{
    AiAppTool, AiChatRequest, AiChatToolCall, AiChatToolResult, AiInference, AiInferenceRequest,
    AiToolTurn,
};

// ---------------------------------------------------------------------------
// Response shape
// ---------------------------------------------------------------------------

/// Token / unit accounting for a single call.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub units: u64,
}

/// Snapshot of the app's budget *after* this call.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub period: String,
    pub units_used: u64,
    pub units_cap: u64,
}

/// Safety / review metadata.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Review {
    pub safety: String,
    pub prompt_redacted: bool,
}

/// A proposed call to an App function, not authorization to execute it.
pub type ProposedToolCall = AiChatToolCall;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse<T> {
    pub provider: String,
    pub model: String,
    pub units: u64,
    pub output: T,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatOutput {
    pub text: String,
    pub tool_calls: Vec<ProposedToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub vector: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Speech {
    pub format: String,
    #[serde(rename = "audio_base64", with = "base64_bytes")]
    pub audio: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate_hz: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcription {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub format: String,
    #[serde(rename = "image_base64", with = "base64_bytes")]
    pub image: Vec<u8>,
}

/// Media bytes carried as standard padded base64 on the wire.
mod base64_bytes {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{de::Error, Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        STANDARD.decode(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub answers: serde_json::Map<String, serde_json::Value>,
}

/// Parsed reply from `yu ai chat`. See `wire/v1/ai.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiResponse {
    pub text: String,
    pub model: String,
    pub provider: String,
    #[serde(skip)]
    pub units: u64,
    #[serde(skip)]
    pub output: ChatOutput,
    pub verb: String,
    pub usage: Usage,
    pub budget: Budget,
    pub review: Review,
    #[serde(default)]
    pub tool_calls: Vec<ProposedToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_state: Option<String>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("ai: {0}")]
    InvalidArg(String),

    /// The App does not have permission to make the call.
    #[error("ai denied: {message}")]
    Denied {
        message: String,
        code: String,
        payload: serde_json::Value,
    },

    /// Available budget could not cover the call.
    #[error("budget exceeded: {0}")]
    BudgetExceeded(String),

    /// Safety pipeline blocked the call.
    #[error("safety violation: {0}")]
    SafetyViolation(String),

    /// Transport / decode failure.
    #[error("ai unavailable: {0}")]
    Unavailable(String),

    /// Yu OS returned `KERNEL_UNAVAILABLE`.
    #[error("AI service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error(transparent)]
    Bridge(#[from] BridgeError),
}

// ---------------------------------------------------------------------------
// Options builder
// ---------------------------------------------------------------------------

/// Options for [`chat`].
#[derive(Debug, Default, Clone)]
pub struct ChatOpts {
    origin: Option<String>,
    max_units: Option<u64>,
    system: Option<String>,
    app_id: Option<String>,
    tools: Option<Vec<String>>,
    images: Option<Vec<Vec<u8>>>,
    functions: Option<Vec<AiAppTool>>,
    tool_turns: Option<Vec<AiToolTurn>>,
}

impl ChatOpts {
    /// Select the explicit origin route used by older Apps.
    pub fn origin(mut self, o: impl Into<String>) -> Self {
        self.origin = Some(o.into());
        self
    }
    pub fn max_units(mut self, n: u64) -> Self {
        self.max_units = Some(n);
        self
    }
    /// Optional App instruction, not the system Agent's prompt.
    pub fn system(mut self, s: impl Into<String>) -> Self {
        self.system = Some(s.into());
        self
    }
    pub fn app(mut self, id: impl Into<String>) -> Self {
        self.app_id = Some(id.into());
        self
    }
    /// Attach PNG, JPEG or WebP image bytes; Yu OS detects each format.
    pub fn images(mut self, images: Vec<Vec<u8>>) -> Self {
        self.images = (!images.is_empty()).then_some(images);
        self
    }
    pub fn functions(mut self, functions: Vec<AiAppTool>) -> Self {
        self.functions = (!functions.is_empty()).then_some(functions);
        self
    }
    pub fn tool_turns(mut self, tool_turns: Vec<AiToolTurn>) -> Self {
        self.tool_turns = (!tool_turns.is_empty()).then_some(tool_turns);
        self
    }
    /// Legacy catalog names are not accepted. Use [`chat_request`] for App functions.
    pub fn tools<I, S>(mut self, tools: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tools = Some(tools.into_iter().map(Into::into).collect());
        self
    }
}

// ---------------------------------------------------------------------------
// Public helpers
// ---------------------------------------------------------------------------

mod stream;

pub use stream::chat_stream;

/// Call the configured chat model.
pub fn chat(prompt: &str, opts: ChatOpts) -> Result<AiResponse, AiError> {
    if prompt.trim().is_empty() {
        return Err(AiError::InvalidArg("chat: prompt must be non-empty".into()));
    }
    if opts.origin.as_ref().is_some_and(|origin| origin.trim().is_empty()) {
        return Err(AiError::InvalidArg("chat: invalid origin".into()));
    }
    let is_dev = dev_ai_url().is_some();
    if opts.origin.is_none() || opts.images.is_some() || opts.functions.is_some() || opts.tool_turns.is_some() || is_dev {
        if opts.tools.as_ref().is_some_and(|tools| !tools.is_empty())
            && (opts.origin.is_none() || opts.images.is_some() || opts.functions.is_some() || opts.tool_turns.is_some())
        {
            return Err(AiError::InvalidArg("chat: supply App functions in functions".into()));
        }
        return chat_request(&structured_chat_request(prompt, opts)?);
    }
    dispatch(prompt, &opts)
}

fn structured_chat_request(prompt: &str, opts: ChatOpts) -> Result<AiChatRequest, AiError> {
    let env_app = std::env::var("YUOS_APP_ID").ok().filter(|s| !s.is_empty());
    if let Some(app) = opts.app_id.as_ref() {
        if (env_app.is_some() || dev_ai_url().is_none()) && env_app.as_ref() != Some(app) {
            return Err(AiError::Denied {
                message: "chat: App identity does not match the session".into(),
                code: "PERMISSION_DENIED".into(),
                payload: serde_json::json!({"code": "PERMISSION_DENIED", "error": "chat: App identity does not match the session"}),
            });
        }
    }
    Ok(AiChatRequest {
        origin: opts.origin.unwrap_or_else(|| "app".into()),
        prompt: prompt.into(),
        system: opts.system,
        images: opts.images.map(|images| images.iter().map(|image| STANDARD.encode(image)).collect()),
        max_units: opts.max_units,
        tools: opts.functions,
        tool_turns: opts.tool_turns,
    })
}

/// Call the configured chat model with optional App functions and prior results.
pub fn chat_request(request: &AiChatRequest) -> Result<AiResponse, AiError> {
    let input = encode_chat_request(request)?;
    if dev_ai_url().is_some() {
        return dev_chat(request);
    }
    chat_wire_reply(crate::yu_call_json_structured_input(
        "ai", "chat", ["ai", "chat", "--request-stdin"], Some(&input),
        std::process::Stdio::piped(), None,
    ))
}

fn encode_chat_request(request: &AiChatRequest) -> Result<Vec<u8>, AiError> {
    let value = serde_json::to_value(request)
        .map_err(|error| AiError::InvalidArg(error.to_string()))?;
    let input = serde_json::to_vec(&value)
        .map_err(|error| AiError::InvalidArg(error.to_string()))?;
    if input.len() > 720 * 1024 {
        return Err(AiError::InvalidArg("App chat request exceeds the byte limit".into()));
    }
    crate::generated::validate_ai_chat_request(&value)
        .map_err(|error| AiError::InvalidArg(error.to_string()))?;
    Ok(input)
}

fn chat_wire_reply(reply: Result<serde_json::Value, StructuredCosError>) -> Result<AiResponse, AiError> {
    match reply {
        Ok(reply) => parse_response(reply),
        Err(StructuredCosError::App(error)) =>
            Err(classify_ai_error(&error.message, &error.payload)),
        Err(StructuredCosError::Bridge(error)) => Err(AiError::Bridge(error)),
    }
}

/// Run the owner's configured embedding, speech, image, or decision model.
pub fn infer(request: &AiInferenceRequest) -> Result<AiInference, AiError> {
    let value = serde_json::to_value(request)
        .map_err(|error| AiError::InvalidArg(error.to_string()))?;
    let input = serde_json::to_vec(&value)
        .map_err(|error| AiError::InvalidArg(error.to_string()))?;
    if input.len() > 720 * 1024 {
        return Err(AiError::InvalidArg("AI request exceeds the byte limit".into()));
    }
    crate::generated::validate_ai_inference_request(&value)
        .map_err(|error| AiError::InvalidArg(error.to_string()))?;
    let mut reply = if dev_ai_url().is_some() {
        dev_infer(&value)?
    } else {
        let response = crate::yu_call_json_structured_input(
            "ai", "infer", ["ai", "infer", "--request-stdin"], Some(&input),
            std::process::Stdio::piped(), None,
        );
        match response {
            Ok(reply) => reply,
            Err(StructuredCosError::App(error)) =>
                return Err(classify_ai_error(&error.message, &error.payload)),
            Err(StructuredCosError::Bridge(error)) => return Err(AiError::Bridge(error)),
        }
    };
    crate::generated::validate_ai_inference(&reply)
        .map_err(|error| AiError::Unavailable(error.to_string()))?;
    if reply["output"]["kind"] != value["input"]["kind"] {
        return Err(AiError::Unavailable("AI inference reply does not match the request".into()));
    }
    crate::generated::normalize_ai_inference_integers(&mut reply);
    serde_json::from_value(reply).map_err(|error| AiError::Unavailable(error.to_string()))
}

#[derive(Debug, Default, Clone)]
pub struct ModelOpts {
    pub origin: String,
    pub max_units: Option<u32>,
}

impl ModelOpts {
    pub fn new(origin: impl Into<String>) -> Self {
        Self { origin: origin.into(), max_units: None }
    }
}

fn run_model<T: serde::de::DeserializeOwned>(
    input: serde_json::Value,
    opts: ModelOpts,
) -> Result<ModelResponse<T>, AiError> {
    let origin = if opts.origin.is_empty() { "app".into() } else { opts.origin };
    let reply = infer(&AiInferenceRequest { origin, max_units: opts.max_units, input })?;
    let output = serde_json::from_value(reply.output)
        .map_err(|error| AiError::Unavailable(error.to_string()))?;
    Ok(ModelResponse { provider: reply.provider, model: reply.model, units: reply.units.into(), output })
}

pub fn embed(text: &str, opts: ModelOpts) -> Result<ModelResponse<Embedding>, AiError> {
    run_model(serde_json::json!({"kind": "embed", "text": text}), opts)
}

pub fn tts(text: &str, opts: ModelOpts) -> Result<ModelResponse<Speech>, AiError> {
    run_model(serde_json::json!({"kind": "tts", "text": text}), opts)
}

/// Transcribe WAV, MP3 or Ogg audio bytes; Yu OS detects the format.
pub fn stt(audio: &[u8], opts: ModelOpts) -> Result<ModelResponse<Transcription>, AiError> {
    run_model(serde_json::json!({"kind": "stt", "audio_base64": STANDARD.encode(audio)}), opts)
}

pub fn imagegen(prompt: &str, opts: ModelOpts) -> Result<ModelResponse<Image>, AiError> {
    run_model(serde_json::json!({"kind": "imagegen", "prompt": prompt}), opts)
}

pub fn decide(
    state: serde_json::Value,
    questions: serde_json::Map<String, serde_json::Value>,
    opts: ModelOpts,
) -> Result<ModelResponse<Decision>, AiError> {
    run_model(serde_json::json!({"kind": "decision", "state": state, "questions": questions}), opts)
}

/// Open Applications settings after a user action.
pub fn open_permission_settings() -> Result<(), AiError> {
    match yu_call_json_structured("ai", "settings", ["ai", "settings"]) {
        Ok(_) => Ok(()),
        Err(StructuredCosError::App(error)) =>
            Err(classify_ai_error(&error.message, &error.payload)),
        Err(StructuredCosError::Bridge(error)) => Err(AiError::Bridge(error)),
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

struct PrivateInputFile {
    path: PathBuf,
}

impl PrivateInputFile {
    fn new(label: &str, contents: &str) -> Result<Self, AiError> {
        static NEXT_FILE: AtomicU64 = AtomicU64::new(0);
        let sequence = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "yuos-ai-{}-{nonce}-{sequence}-{label}",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&path)
            .map_err(|error| AiError::Unavailable(format!("create private {label}: {error}")))?;
        if let Err(error) = file
            .write_all(contents.as_bytes())
            .and_then(|_| file.sync_all())
        {
            drop(file);
            let _ = std::fs::remove_file(&path);
            return Err(AiError::Unavailable(format!(
                "write private {label}: {error}"
            )));
        }
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PrivateInputFile {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.path) {
            eprintln!("yuos-sdk: failed to remove private AI input file: {error}");
        }
    }
}

fn dispatch(prompt: &str, opts: &ChatOpts) -> Result<AiResponse, AiError> {
    let app = opts
        .app_id
        .clone()
        .or_else(|| std::env::var("YUOS_APP_ID").ok().filter(|s| !s.is_empty()))
        .or_else(|| dev_ai_url().map(|_| "dev".to_string()))
        .ok_or_else(|| {
            classify_ai_error(
                "chat: App identity is required",
                &serde_json::json!({"code": "PERMISSION_DENIED"}),
            )
        })?;

    let mut argv: Vec<OsString> = vec![
        "ai".into(),
        "chat".into(),
        "--app".into(),
        app.into(),
        "--origin".into(),
        opts.origin.as_deref().unwrap_or("app").into(),
    ];
    let prompt_file = PrivateInputFile::new("prompt", prompt)?;
    argv.push("--prompt-file".into());
    argv.push(prompt_file.path().as_os_str().to_owned());
    if let Some(n) = opts.max_units {
        argv.push("--max-units".into());
        argv.push(n.to_string().into());
    }
    let system_file = opts
        .system
        .as_deref()
        .map(|system| PrivateInputFile::new("system", system))
        .transpose()?;
    if let Some(file) = &system_file {
        argv.push("--system-file".into());
        argv.push(file.path().as_os_str().to_owned());
    }
    if let Some(tools) = &opts.tools {
        argv.push("--tools".into());
        argv.push(tools.join(",").into());
    }

    let value = match yu_call_json_structured("ai", "chat", argv) {
        Ok(value) => value,
        Err(StructuredCosError::App(error)) => {
            let error = *error;
            let message = error.payload["error"]
                .as_str()
                .unwrap_or("AI request denied");
            return Err(classify_ai_error(message, &error.payload));
        }
        Err(StructuredCosError::Bridge(error)) => return Err(AiError::Bridge(error)),
    };
    parse_response(value)
}

fn parse_response(mut value: serde_json::Value) -> Result<AiResponse, AiError> {
    crate::generated::validate_ai(&value)
        .map_err(|error| AiError::Unavailable(format!("ai response decode failed: {error}")))?;
    crate::generated::normalize_ai_integers(&mut value);
    let mut resp: AiResponse = serde_json::from_value(value)
        .map_err(|e| AiError::Unavailable(format!("ai response decode failed: {e}")))?;
    resp.units = resp.usage.units;
    resp.output = ChatOutput {
        text: resp.text.clone(),
        tool_calls: resp.tool_calls.clone(),
        provider_state: resp.provider_state.clone(),
    };
    Ok(resp)
}

/// Best-effort redaction of an AI request/response payload for use in
/// error messages and Debug output. Replaces the bulky text /
/// embedding / image-bytes fields with their byte counts so the
/// surrounding diagnostic stays useful without leaking content.
fn redact_payload(value: &serde_json::Value) -> serde_json::Value {
    let mut redacted = value.clone();
    if let Some(obj) = redacted.as_object_mut() {
        for key in [
            "text",
            "prompt",
            "messages",
            "embedding",
            "embeddings",
            "data",
            "raw",
        ] {
            if let Some(v) = obj.get_mut(key) {
                let bytes = serde_json::to_string(v).map(|s| s.len()).unwrap_or(0);
                *v = serde_json::json!({
                    "redacted": true,
                    "approx_bytes": bytes,
                });
            }
        }
    }
    redacted
}

fn classify_ai_error(message: &str, payload: &serde_json::Value) -> AiError {
    let code = payload
        .get("code")
        .and_then(|v| v.as_str())
        .expect("validated wire error envelope must include code")
        .to_string();
    match code.as_str() {
        "BUDGET_EXCEEDED" => return AiError::BudgetExceeded(message.to_string()),
        "SAFETY_VIOLATION" => return AiError::SafetyViolation(message.to_string()),
        "KERNEL_UNAVAILABLE" => return AiError::ServiceUnavailable(message.to_string()),
        "INVALID_ARGS" => return AiError::InvalidArg(message.to_string()),
        "PERMISSION_DENIED" | "UNKNOWN_APP" | "UNKNOWN_VERB" | "DENIED" => {}
        _ => return AiError::Bridge(BridgeError::AppError {
            app: "ai".into(),
            verb: "model".into(),
            message: message.into(),
            code,
        }),
    }
    AiError::Denied {
        message: message.to_string(),
        code,
        // Never embed the raw kernel reply here: AI calls routinely
        // include user text, embedding vectors, or image bytes that
        // logs or error toasts would happily render. Redact to byte
        // counts so the structural diagnostic survives without the
        // payload.
        payload: redact_payload(payload),
    }
}

fn dev_ai_url() -> Option<String> {
    if std::env::var("YUOS_SESSION")
        .ok()
        .is_some_and(|s| !s.trim().is_empty())
    {
        return None;
    }
    std::env::var("YUOS_DEV_AI_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
}

fn dev_ai_model(kind: &str) -> String {
    let (env_var, default_model) = match kind {
        "embed" => ("YUOS_DEV_AI_EMBED_MODEL", "text-embedding-3-small"),
        "tts" => ("YUOS_DEV_AI_TTS_MODEL", "tts-1"),
        "stt" => ("YUOS_DEV_AI_STT_MODEL", "whisper-1"),
        "imagegen" => ("YUOS_DEV_AI_IMAGEGEN_MODEL", "dall-e-3"),
        "decision" | "decide" => ("YUOS_DEV_AI_DECIDE_MODEL", "gpt-6-sol"),
        _ => ("YUOS_DEV_AI_CHAT_MODEL", "gpt-6-sol"),
    };
    std::env::var(env_var)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::env::var("YUOS_DEV_AI_MODEL")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| default_model.to_string())
}

fn dev_ai_key() -> Option<String> {
    std::env::var("YUOS_DEV_AI_KEY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn decode_chunked(mut input: &[u8]) -> Result<Vec<u8>, AiError> {
    let mut out = Vec::new();
    loop {
        let line_end = input
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or_else(|| AiError::Unavailable("malformed chunked response".into()))?;
        let size_str = std::str::from_utf8(&input[..line_end])
            .map_err(|e| AiError::Unavailable(e.to_string()))?
            .split(';')
            .next()
            .unwrap_or("")
            .trim();
        let size = usize::from_str_radix(size_str, 16)
            .map_err(|e| AiError::Unavailable(e.to_string()))?;
        input = &input[line_end + 2..];
        if size == 0 {
            break;
        }
        if input.len() < size + 2 {
            return Err(AiError::Unavailable("truncated chunked response".into()));
        }
        out.extend_from_slice(&input[..size]);
        input = &input[size + 2..];
    }
    Ok(out)
}

fn dev_post_bytes(path: &str, payload: &serde_json::Value) -> Result<Vec<u8>, AiError> {
    let base_url = dev_ai_url()
        .ok_or_else(|| AiError::Unavailable("local dev AI URL is not configured".into()))?;
    let url = format!("{base_url}{path}");
    let body = serde_json::to_vec(payload)
        .map_err(|error| AiError::InvalidArg(error.to_string()))?;
    if url.starts_with("https://") {
        let mut cmd = std::process::Command::new("curl");
        cmd.args([
            "-sS",
            "-f",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
        ]);
        if let Some(key) = dev_ai_key() {
            cmd.args(["-H", &format!("Authorization: Bearer {key}")]);
        }
        cmd.args(["--data-binary", "@-", &url]);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        let mut child = cmd
            .spawn()
            .map_err(|error| AiError::Unavailable(format!("dev AI curl failed: {error}")))?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(&body);
        }
        let output = child
            .wait_with_output()
            .map_err(|error| AiError::Unavailable(format!("dev AI curl failed: {error}")))?;
        if !output.status.success() {
            return Err(AiError::Unavailable(format!(
                "dev AI HTTPS request failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        return Ok(output.stdout);
    }

    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| AiError::Unavailable(format!("unsupported dev AI URL scheme: {url}")))?;
    let (authority, req_path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };
    let connect_addr = if authority.starts_with('[') {
        if authority.contains("]:") {
            authority.to_string()
        } else {
            format!("{authority}:80")
        }
    } else if authority.contains(':') {
        authority.to_string()
    } else {
        format!("{authority}:80")
    };

    let mut stream = TcpStream::connect(&connect_addr)
        .map_err(|error| AiError::Unavailable(format!("dev AI connect failed: {error}")))?;
    // Model replies have no SDK deadline.
    let _ = stream.set_write_timeout(Some(Duration::from_secs(60)));

    let mut req_head = format!(
        "POST {req_path} HTTP/1.1\r\nHost: {authority}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(key) = dev_ai_key() {
        req_head.push_str(&format!("Authorization: Bearer {key}\r\n"));
    }
    req_head.push_str("\r\n");

    stream
        .write_all(req_head.as_bytes())
        .and_then(|_| stream.write_all(&body))
        .and_then(|_| stream.flush())
        .map_err(|error| AiError::Unavailable(format!("dev AI write failed: {error}")))?;

    let mut raw_resp = Vec::new();
    stream
        .read_to_end(&mut raw_resp)
        .map_err(|error| AiError::Unavailable(format!("dev AI read failed: {error}")))?;

    let (header_bytes, body_bytes) = if let Some(pos) = raw_resp.windows(4).position(|w| w == b"\r\n\r\n") {
        (&raw_resp[..pos], &raw_resp[pos + 4..])
    } else if let Some(pos) = raw_resp.windows(2).position(|w| w == b"\n\n") {
        (&raw_resp[..pos], &raw_resp[pos + 2..])
    } else {
        return Err(AiError::Unavailable("dev AI returned malformed HTTP response".into()));
    };

    let header_str = std::str::from_utf8(header_bytes)
        .map_err(|error| AiError::Unavailable(format!("dev AI invalid HTTP headers: {error}")))?;
    let mut lines = header_str.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| AiError::Unavailable("dev AI empty HTTP status line".into()))?;
    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| AiError::Unavailable(format!("dev AI invalid status line: {status_line}")))?
        .parse()
        .map_err(|_| AiError::Unavailable(format!("dev AI invalid status code: {status_line}")))?;
    if !(200..300).contains(&status_code) {
        return Err(AiError::Unavailable(format!("dev AI HTTP {status_code}")));
    }

    let is_chunked = lines.any(|line| {
        if let Some((name, val)) = line.split_once(':') {
            name.trim().eq_ignore_ascii_case("transfer-encoding")
                && val.to_ascii_lowercase().contains("chunked")
        } else {
            false
        }
    });

    if is_chunked {
        decode_chunked(body_bytes)
    } else {
        Ok(body_bytes.to_vec())
    }
}

fn dev_post_json(path: &str, payload: &serde_json::Value) -> Result<serde_json::Value, AiError> {
    let raw = dev_post_bytes(path, payload)?;
    let parsed: serde_json::Value = serde_json::from_slice(&raw)
        .map_err(|error| AiError::Unavailable(format!("dev AI returned invalid JSON: {error}")))?;
    if !parsed.is_object() {
        return Err(AiError::Unavailable("dev AI response must be a JSON object".into()));
    }
    Ok(parsed)
}

fn dev_chat_body(request: &AiChatRequest) -> Result<serde_json::Value, AiError> {
    let mut messages = Vec::new();
    if let Some(system) = &request.system {
        messages.push(serde_json::json!({"role": "system", "content": system}));
    }
    if let Some(images) = &request.images {
        if !images.is_empty() {
            let mut parts = vec![serde_json::json!({"type": "text", "text": request.prompt})];
            for b64 in images {
                parts.push(serde_json::json!({
                    "type": "image_url",
                    "image_url": {"url": format!("data:image/png;base64,{b64}")}
                }));
            }
            messages.push(serde_json::json!({"role": "user", "content": parts}));
        } else {
            messages.push(serde_json::json!({"role": "user", "content": request.prompt}));
        }
    } else {
        messages.push(serde_json::json!({"role": "user", "content": request.prompt}));
    }
    if let Some(turns) = &request.tool_turns {
        for turn in turns {
            let mut tool_calls_out = Vec::with_capacity(turn.calls.len());
            for c in &turn.calls {
                let args_str = serde_json::to_string(&c.input)
                    .map_err(|error| AiError::InvalidArg(error.to_string()))?;
                tool_calls_out.push(serde_json::json!({
                    "id": c.id,
                    "type": "function",
                    "function": {"name": c.name, "arguments": args_str}
                }));
            }
            let mut assistant_msg = serde_json::json!({"role": "assistant", "tool_calls": tool_calls_out});
            if let Some(text) = &turn.text {
                assistant_msg["content"] = serde_json::Value::String(text.clone());
            }
            messages.push(assistant_msg);
            for r in &turn.results {
                messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": r.id,
                    "content": r.content
                }));
            }
        }
    }
    let mut body = serde_json::json!({"model": dev_ai_model("chat"), "messages": messages});
    if let Some(tools) = &request.tools {
        if !tools.is_empty() {
            let tools_out: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters
                        }
                    })
                })
                .collect();
            body["tools"] = serde_json::Value::Array(tools_out);
        }
    }
    if let Some(max_units) = request.max_units {
        body["max_tokens"] = serde_json::json!(max_units);
    }
    Ok(body)
}

fn dev_chat(request: &AiChatRequest) -> Result<AiResponse, AiError> {
    let resp = dev_post_json("/chat/completions", &dev_chat_body(request)?)?;
    let msg = resp
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("message"));
    let text = msg
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();

    let mut parsed_tool_calls = Vec::new();
    if let Some(raw_calls) = msg.and_then(|m| m.get("tool_calls")).and_then(|tc| tc.as_array()) {
        for (idx, tc) in raw_calls.iter().enumerate() {
            let fn_obj = tc
                .get("function")
                .and_then(|f| f.as_object())
                .ok_or_else(|| AiError::Unavailable("dev AI tool_call missing function".into()))?;
            let name = fn_obj
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| AiError::Unavailable("dev AI tool_call missing function name".into()))?;
            let decoded_args = match fn_obj.get("arguments") {
                Some(serde_json::Value::String(s)) => {
                    if s.trim().is_empty() {
                        serde_json::json!({})
                    } else {
                        serde_json::from_str(s).map_err(|error| {
                            AiError::Unavailable(format!("dev AI tool_call arguments invalid JSON: {error}"))
                        })?
                    }
                }
                Some(other) => other.clone(),
                None => serde_json::json!({}),
            };
            let id = tc
                .get("id")
                .and_then(|i| i.as_str())
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("call_{idx}"));
            parsed_tool_calls.push(serde_json::json!({
                "id": id,
                "name": name,
                "input": decoded_args
            }));
        }
    }

    dev_chat_reply(
        resp.get("model").and_then(|m| m.as_str()),
        text,
        resp.get("usage"),
        parsed_tool_calls,
    )
}

fn dev_chat_reply(
    model: Option<&str>,
    text: String,
    usage: Option<&serde_json::Value>,
    tool_calls: Vec<serde_json::Value>,
) -> Result<AiResponse, AiError> {
    let total_tokens = usage
        .and_then(|u| u.get("total_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1);
    let prompt_tokens = usage
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let completion_tokens = usage
        .and_then(|u| u.get("completion_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let actual_model = model
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| dev_ai_model("chat"));

    let mut env = serde_json::json!({
        "text": text,
        "model": actual_model,
        "provider": "dev",
        "verb": "ai.chat",
        "usage": {
            "input_tokens": prompt_tokens,
            "output_tokens": completion_tokens,
            "units": total_tokens
        },
        "budget": {
            "period": "dev",
            "units_used": 0,
            "units_cap": 0
        },
        "review": {
            "safety": "strict",
            "prompt_redacted": false
        }
    });
    if !tool_calls.is_empty() {
        env["tool_calls"] = serde_json::Value::Array(tool_calls);
    }
    parse_response(env)
}

fn dev_infer(request: &serde_json::Value) -> Result<serde_json::Value, AiError> {
    let inp = request
        .get("input")
        .ok_or_else(|| AiError::InvalidArg("missing inference input".into()))?;
    let kind = inp
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| AiError::InvalidArg("missing inference input kind".into()))?;
    let model = dev_ai_model(kind);
    let mut units: u64 = 1;
    let mut actual_model = model.clone();

    let output = match kind {
        "embed" => {
            let text = inp.get("text").and_then(|t| t.as_str()).unwrap_or("");
            let resp = dev_post_json("/embeddings", &serde_json::json!({"model": model, "input": text}))?;
            let embedding = resp
                .get("data")
                .and_then(|d| d.as_array())
                .and_then(|arr| arr.first())
                .and_then(|first| first.get("embedding"))
                .and_then(|e| e.as_array())
                .ok_or_else(|| AiError::Unavailable("dev AI embeddings response missing data[0].embedding".into()))?;
            if let Some(m) = resp.get("model").and_then(|m| m.as_str()).map(str::trim).filter(|s| !s.is_empty()) {
                actual_model = m.to_string();
            }
            if let Some(u) = resp.get("usage").and_then(|u| u.get("total_tokens")).and_then(|t| t.as_u64()).filter(|&u| u >= 1) {
                units = u;
            }
            serde_json::json!({"kind": "embed", "vector": embedding})
        }
        "tts" => {
            let text = inp.get("text").and_then(|t| t.as_str()).unwrap_or("");
            let raw = dev_post_bytes(
                "/audio/speech",
                &serde_json::json!({"model": model, "input": text, "voice": "alloy", "response_format": "wav"}),
            )?;
            let mut audio_b64: Option<String> = None;
            let mut format = "wav".to_string();
            if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&raw) {
                if let Some(b64) = parsed.get("audio_base64").and_then(|v| v.as_str()) {
                    audio_b64 = Some(b64.to_string());
                    if let Some(f) = parsed.get("format").and_then(|v| v.as_str()).filter(|s| !s.is_empty()) {
                        format = f.to_string();
                    }
                }
            }
            let audio_b64 = match audio_b64 {
                Some(b64) => b64,
                None => {
                    if raw.is_empty() {
                        return Err(AiError::Unavailable("dev AI speech response is empty".into()));
                    }
                    STANDARD.encode(&raw)
                }
            };
            serde_json::json!({"kind": "tts", "format": format, "audio_base64": audio_b64})
        }
        "stt" => {
            let audio_base64 = inp.get("audio_base64").and_then(|a| a.as_str()).unwrap_or("");
            let resp = dev_post_json(
                "/audio/transcriptions",
                &serde_json::json!({"model": model, "audio_base64": audio_base64}),
            )?;
            let text = resp
                .get("text")
                .and_then(|t| t.as_str())
                .ok_or_else(|| AiError::Unavailable("dev AI transcription response missing text".into()))?;
            serde_json::json!({"kind": "stt", "text": text})
        }
        "imagegen" => {
            let prompt = inp.get("prompt").and_then(|p| p.as_str()).unwrap_or("");
            let resp = dev_post_json(
                "/images/generations",
                &serde_json::json!({"model": model, "prompt": prompt, "response_format": "b64_json"}),
            )?;
            let b64_json = resp
                .get("data")
                .and_then(|d| d.as_array())
                .and_then(|arr| arr.first())
                .and_then(|first| first.get("b64_json"))
                .and_then(|b| b.as_str())
                .ok_or_else(|| AiError::Unavailable("dev AI imagegen response missing data[0].b64_json".into()))?;
            serde_json::json!({"kind": "imagegen", "format": "png", "image_base64": b64_json})
        }
        "decision" | "decide" => {
            let user_payload = serde_json::to_string(&serde_json::json!({
                "state": inp.get("state").cloned().unwrap_or(serde_json::Value::Null),
                "questions": inp.get("questions").cloned().unwrap_or_else(|| serde_json::json!({}))
            }))
            .map_err(|error| AiError::InvalidArg(error.to_string()))?;
            let resp = dev_post_json(
                "/chat/completions",
                &serde_json::json!({
                    "model": model,
                    "response_format": {"type": "json_object"},
                    "messages": [
                        {"role": "system", "content": "Return JSON {\"answers\": {...}} answering each question."},
                        {"role": "user", "content": user_payload}
                    ]
                }),
            )?;
            let content = resp
                .get("choices")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|first| first.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .ok_or_else(|| AiError::Unavailable("dev AI decision response missing message content".into()))?;
            let parsed_content: serde_json::Value = serde_json::from_str(content)
                .map_err(|error| AiError::Unavailable(format!("dev AI decision content is not valid JSON: {error}")))?;
            let answers = parsed_content
                .get("answers")
                .filter(|a| a.is_object())
                .ok_or_else(|| AiError::Unavailable("dev AI decision content missing answers object".into()))?;
            serde_json::json!({"kind": kind, "answers": answers})
        }
        other => return Err(AiError::InvalidArg(format!("unsupported inference kind: {other}"))),
    };

    Ok(serde_json::json!({
        "provider": "dev",
        "model": actual_model,
        "units": units,
        "output": output
    }))
}

#[cfg(test)]
mod tests {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/test/unit/ai.rs"));
}
