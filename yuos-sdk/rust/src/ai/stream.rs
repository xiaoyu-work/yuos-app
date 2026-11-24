use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, Command, Output, Stdio},
};

use serde_json::Value;

use super::{
    chat_wire_reply, dev_ai_key, dev_ai_url, dev_chat, dev_chat_body, dev_chat_reply,
    encode_chat_request, structured_chat_request, AiChatRequest, AiError, AiResponse, ChatOpts,
};

const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;

/// Stream text from the configured model and return its final response.
pub fn chat_stream(
    prompt: &str,
    opts: ChatOpts,
    mut on_chunk: impl FnMut(&str),
) -> Result<AiResponse, AiError> {
    if prompt.trim().is_empty() {
        return Err(AiError::InvalidArg("chat: prompt must be non-empty".into()));
    }
    if opts.tools.as_ref().is_some_and(|tools| !tools.is_empty()) {
        return Err(AiError::InvalidArg(
            "chat: supply App functions in functions".into(),
        ));
    }
    let request = structured_chat_request(prompt, opts)?;
    let payload = encode_chat_request(&request)?;
    if dev_ai_url().is_some() {
        return dev_chat_stream(&request, &mut on_chunk);
    }

    let binary = std::env::var_os("YUOS_KERNEL_BIN").unwrap_or_else(|| "yu".into());
    let mut child = Command::new(binary)
        .args(["--wire=1", "ai", "chat", "--request-stdin", "--stream"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(unavailable)?;
    let mut stdin = child.stdin.take().expect("piped chat stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("piped chat stdout"));
    let mut process = KillOnDrop(child);
    if let Err(error) = stdin.write_all(&payload) {
        if error.kind() != std::io::ErrorKind::BrokenPipe {
            return Err(unavailable(error));
        }
    }
    drop(stdin);

    let mut final_line = None;
    let mut frame = Vec::new();
    loop {
        frame.clear();
        if (&mut stdout)
            .take((MAX_LINE_BYTES + 1) as u64)
            .read_until(b'\n', &mut frame)
            .map_err(unavailable)?
            == 0
        {
            break;
        }
        if final_line.is_some() || frame.len() > MAX_LINE_BYTES || !frame.ends_with(b"\n") {
            return Err(unavailable("invalid App chat stream framing"));
        }
        let record: Value = serde_json::from_slice(&frame).map_err(unavailable)?;
        if record.get("event").is_some() {
            crate::generated::validate_ai_chat_delta(&record).map_err(unavailable)?;
            on_chunk(record["text"].as_str().expect("validated chat delta text"));
        } else {
            crate::generated::validate_envelope(&record).map_err(unavailable)?;
            final_line = Some(std::mem::take(&mut frame));
        }
    }
    let stdout =
        final_line.ok_or_else(|| unavailable("App chat stream ended without a final response"))?;
    let status = process.0.wait().map_err(unavailable)?;
    chat_wire_reply(crate::decode_wire_response(
        "ai",
        "chat",
        Output {
            status,
            stdout,
            stderr: Vec::new(),
        },
    ))
}

/// Read the dev server's SSE text deltas through curl. Requests with App
/// functions return the whole reply so tool calls keep one decoder.
fn dev_chat_stream(
    request: &AiChatRequest,
    on_chunk: &mut impl FnMut(&str),
) -> Result<AiResponse, AiError> {
    if request.tools.as_ref().is_some_and(|tools| !tools.is_empty())
        || request.tool_turns.as_ref().is_some_and(|turns| !turns.is_empty())
    {
        let reply = dev_chat(request)?;
        if !reply.output.text.is_empty() {
            on_chunk(&reply.output.text);
        }
        return Ok(reply);
    }
    let mut body = dev_chat_body(request)?;
    body["stream"] = Value::Bool(true);
    let url = format!(
        "{}/chat/completions",
        dev_ai_url().expect("dev streaming requires a dev AI URL")
    );
    let mut curl = Command::new("curl");
    curl.args(["-sS", "-N", "-f", "-X", "POST", "-H", "Content-Type: application/json"]);
    if let Some(key) = dev_ai_key() {
        curl.args(["-H", &format!("Authorization: Bearer {key}")]);
    }
    let mut child = curl
        .args(["--data-binary", "@-", &url])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| unavailable(format!("dev AI curl failed: {error}")))?;
    let mut stdin = child.stdin.take().expect("piped curl stdin");
    let stdout = BufReader::new(child.stdout.take().expect("piped curl stdout"));
    let _process = KillOnDrop(child);
    stdin
        .write_all(&serde_json::to_vec(&body).map_err(unavailable)?)
        .map_err(unavailable)?;
    drop(stdin);

    let mut text = String::new();
    let mut model = None;
    let mut usage = None;
    for line in stdout.lines() {
        let line = line.map_err(unavailable)?;
        let Some(data) = line.trim().strip_prefix("data:").map(str::trim) else {
            continue;
        };
        if data == "[DONE]" {
            return dev_chat_reply(model.as_deref(), text, usage.as_ref(), Vec::new());
        }
        if data.is_empty() {
            continue;
        }
        let chunk: Value = serde_json::from_str(data)
            .map_err(|_| unavailable("dev AI stream could not be decoded"))?;
        if let Some(name) = chunk["model"].as_str().filter(|name| !name.is_empty()) {
            model = Some(name.to_owned());
        }
        if chunk["usage"].is_object() {
            usage = Some(chunk["usage"].clone());
        }
        if let Some(piece) = chunk["choices"][0]["delta"]["content"]
            .as_str()
            .filter(|piece| !piece.is_empty())
        {
            text.push_str(piece);
            on_chunk(piece);
        }
    }
    Err(unavailable("dev AI stream ended without a final event"))
}

fn unavailable(error: impl std::fmt::Display) -> AiError {
    AiError::Unavailable(error.to_string())
}

struct KillOnDrop(Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}