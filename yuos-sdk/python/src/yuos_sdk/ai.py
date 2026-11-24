"""Call the user's configured Yu OS models from an authorized App.

    from yuos_sdk.ai import chat

    summary = chat(body, system="Summarize this document.").output.text
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import urllib.error
import urllib.request
from base64 import b64decode, b64encode
from dataclasses import dataclass, field
from typing import (
    Any, Callable, Dict, Generic, Iterator, List, Literal, Mapping, NoReturn, Optional,
    Sequence, TypeVar, cast,
)


from ._transport import WIRE_ARG, WireDenied, WireUnavailable, decode_response
from .generated import (
    AiAppTool,
    AiChatRequest,
    AiInference,
    AiInferenceRequest,
    AiToolTurn,
    DecisionInput,
    EmbedInput,
    ImageInput,
    SpeechInput,
    TranscriptionInput,
    WireDecodeError,
    WireDecimal,
    WireJsonValue,
    decode_wire_json,
    encode_wire_json,
    validate_ai,
    validate_ai_chat_delta,
    validate_ai_chat_request,
    validate_ai_inference,
    validate_ai_inference_request,
    validate_budget_show,
    validate_envelope,
    wire_integer_to_int,
)


# Timeout for short non-model calls. Model calls have no SDK deadline; Yu OS
# applies the provider's idle timeout instead.
_DEFAULT_TIMEOUT_S = 60


# ---------------------------------------------------------------------------
# Exceptions
# ---------------------------------------------------------------------------


class AiError(Exception):
    """Base class for every error this module raises."""

    def __init__(self, message: str, payload: Optional[Mapping[str, Any]] = None):
        super().__init__(message)
        self.payload = dict(payload) if payload is not None else {}
        self.code = self.payload.get("code")


class AiUnavailable(AiError):
    """The OS AI service or the local SDK transport is unavailable."""


class AiInvalidArgs(AiError):
    """The input did not satisfy the model call's contract."""


class AiDenied(AiError):
    """The App does not have permission to make the call."""

    def __init__(self, payload: Mapping[str, Any]):
        super().__init__(str(payload.get("error") or "AI call denied"), payload)


class AiBudgetExceeded(AiDenied):
    """The available budget could not cover the call."""


class AiSafetyViolation(AiDenied):
    """The safety pipeline refused the request."""


# ---------------------------------------------------------------------------
# Response shape
# ---------------------------------------------------------------------------


@dataclass
class Usage:
    input_tokens: int = 0
    output_tokens: int = 0
    units: int = 0


@dataclass
class Budget:
    period: str = ""
    units_used: int = 0
    units_cap: int = 0


@dataclass
class BudgetUsage:
    app: str
    period: str
    units_used: int


@dataclass
class Review:
    safety: str = "strict"
    prompt_redacted: bool = False


@dataclass
class ProposedToolCall:
    """A proposed call to an App function; only the App decides whether to execute it."""

    id: str
    name: str
    input: WireJsonValue


Origin = Literal["trusted", "user-input", "external-content"]
_Output = TypeVar("_Output")


@dataclass(frozen=True)
class ModelResponse(Generic[_Output]):
    provider: str
    model: str
    units: int
    output: _Output


@dataclass(frozen=True)
class ChatOutput:
    text: str
    tool_calls: List[ProposedToolCall]
    provider_state: Optional[str]


@dataclass(frozen=True)
class Embedding:
    vector: List[float]


@dataclass(frozen=True)
class Speech:
    format: str
    audio: bytes
    sample_rate_hz: Optional[int] = None


@dataclass(frozen=True)
class Transcription:
    text: str


@dataclass(frozen=True)
class Image:
    format: str
    image: bytes


@dataclass(frozen=True)
class Decision:
    answers: Mapping[str, WireJsonValue]


@dataclass
class AiResponse:
    text: str
    model: str
    provider: str
    verb: str = ""
    usage: Usage = field(default_factory=Usage)
    budget: Budget = field(default_factory=Budget)
    review: Review = field(default_factory=Review)
    tool_calls: List[ProposedToolCall] = field(default_factory=list)
    raw: Dict[str, WireJsonValue] = field(default_factory=dict)
    provider_state: Optional[str] = None

    @property
    def units(self) -> int:
        return self.usage.units

    @property
    def output(self) -> ChatOutput:
        return ChatOutput(self.text, self.tool_calls, self.provider_state)


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def chat(
    prompt: str,
    *,
    origin: Optional[Origin] = None,
    max_units: Optional[int] = None,
    system: Optional[str] = None,
    app_id: Optional[str] = None,
    tools: Optional[List[str]] = None,
    images: Sequence[bytes] = (),
    functions: Sequence[AiAppTool] = (),
    tool_turns: Sequence[AiToolTurn] = (),
) -> AiResponse:
    """Call the configured chat model with text, optional images and App functions.

    Pass PNG, JPEG or WebP image bytes; Yu OS detects each format.
    """
    if not isinstance(prompt, str) or not prompt.strip():
        raise AiInvalidArgs("chat: prompt must be non-empty")
    if origin is not None and (not isinstance(origin, str) or not origin.strip()):
        raise AiInvalidArgs("chat: invalid origin")
    if origin is None or images or functions or tool_turns or _dev_ai_url() is not None:
        if tools and (origin is None or images or functions or tool_turns):
            raise AiInvalidArgs("chat: supply App functions in functions")
        env_app = os.environ.get("YUOS_APP_ID")
        if app_id and (env_app or _dev_ai_url() is None) and app_id != env_app:
            raise AiDenied({"code": "PERMISSION_DENIED", "error": "chat: App identity does not match the session"})
        request: AiChatRequest = {"prompt": prompt, "origin": origin or "app"}
        if system is not None:
            request["system"] = system
        if max_units is not None:
            request["max_units"] = max_units
        if images:
            request["images"] = [_encode_media(image, "chat: images") for image in images]
        if functions:
            request["tools"] = list(functions)
        if tool_turns:
            request["tool_turns"] = list(tool_turns)
        return chat_request(request)
    return _dispatch(
        modality="chat",
        prompt=prompt,
        origin=origin,
        max_units=max_units,
        system=system,
        app_id=app_id,
        tools=tools,
    )


def chat_request(request: AiChatRequest) -> AiResponse:
    """Call the configured chat model with optional App functions and prior results."""
    data = _encode_request(request, validate_ai_chat_request)
    if _dev_ai_url() is not None:
        return _dev_chat(request)
    return _parse_response(_model_call("chat", "--request-stdin", data=data))


def chat_stream(
    prompt: str,
    *,
    origin: Optional[Origin] = None,
    max_units: Optional[int] = None,
    system: Optional[str] = None,
    images: Sequence[bytes] = (),
) -> Iterator[str]:
    """Stream text chunks from the configured chat model."""
    if not isinstance(prompt, str) or not prompt.strip():
        raise AiInvalidArgs("chat_stream: prompt must be non-empty")
    if origin is not None and (not isinstance(origin, str) or not origin.strip()):
        raise AiInvalidArgs("chat_stream: invalid origin")
    request: AiChatRequest = {"prompt": prompt, "origin": origin or "app"}
    if system is not None:
        request["system"] = system
    if max_units is not None:
        request["max_units"] = max_units
    if images:
        request["images"] = [_encode_media(image, "chat_stream: images") for image in images]
    data = _encode_request(request, validate_ai_chat_request)
    if _dev_ai_url() is not None:
        yield from _dev_chat_stream(request)
        return
    yield from _model_stream(data)


def _model_stream(data: bytes) -> Iterator[str]:
    argv = [_yu_binary(), WIRE_ARG, "ai", "chat", "--request-stdin", "--stream"]
    try:
        process = subprocess.Popen(
            argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
        )
    except OSError as error:
        raise AiUnavailable(str(error)) from error
    try:
        assert process.stdin is not None and process.stdout is not None
        try:
            with process.stdin as pipe:
                pipe.write(data)
        except BrokenPipeError:
            # The CLI can send a structured refusal without reading the request.
            pass
        final = None
        while raw := process.stdout.readline(8 * 1024 * 1024 + 1):
            if final is not None or len(raw) > 8 * 1024 * 1024 or not raw.endswith(b"\n"):
                raise AiUnavailable("invalid App chat stream framing")
            text = raw.decode("utf-8")
            record = decode_wire_json(text)
            if isinstance(record, dict) and "event" in record:
                validate_ai_chat_delta(record)
                yield record["text"]
            else:
                validate_envelope(record)
                final = text
        status = process.wait()
        if final is None:
            raise AiUnavailable("App chat stream ended without a final response")
        _parse_response(decode_response(final, status, "yu ai chat"))
    except WireDenied as error:
        _raise_for_error(error.payload)
    except (WireDecodeError, WireUnavailable, OSError, UnicodeError, ValueError) as error:
        raise AiUnavailable(str(error)) from error
    finally:
        if process.poll() is None:
            process.kill()
        process.wait()
        if process.stdout is not None:
            process.stdout.close()


def infer(request: AiInferenceRequest) -> AiInference:
    """Run the owner's configured embedding, speech, image, or decision model."""
    data = _encode_request(request, validate_ai_inference_request)
    if _dev_ai_url() is not None:
        reply = _dev_infer(request)
    else:
        reply = _model_call("infer", "--request-stdin", data=data)
    try:
        validate_ai_inference(reply)
    except WireDecodeError as error:
        raise AiUnavailable(str(error)) from error
    if reply["output"]["kind"] != request["input"]["kind"]:
        raise AiUnavailable("AI inference reply does not match the request")
    reply["units"] = wire_integer_to_int(reply["units"])
    return cast(AiInference, reply)


def _encode_request(request: Any, validate: Callable[[Any], Any]) -> bytes:
    try:
        validate(request)
        data = encode_wire_json(request).encode("utf-8")
    except (TypeError, ValueError) as error:
        raise AiInvalidArgs(str(error)) from error
    if len(data) > 720 * 1024:
        raise AiInvalidArgs("AI request exceeds the byte limit")
    return data


def _encode_media(data: Any, label: str) -> str:
    if not isinstance(data, (bytes, bytearray, memoryview)):
        raise AiInvalidArgs(f"{label} must be bytes")
    return b64encode(data).decode("ascii")


def _decode_media(value: str) -> bytes:
    try:
        return b64decode(value, validate=True)
    except ValueError as error:
        raise AiUnavailable(f"AI inference reply has invalid media: {error}") from error


def _run_model(
    input: EmbedInput | SpeechInput | TranscriptionInput | ImageInput | DecisionInput,
    origin: Optional[Origin],
    max_units: Optional[int],
    output_type: Callable[..., _Output],
) -> ModelResponse[_Output]:
    request: AiInferenceRequest = {"origin": "app" if origin is None else origin, "input": input}
    if max_units is not None:
        request["max_units"] = max_units
    reply = infer(request)
    output: Dict[str, Any] = {key: value for key, value in reply["output"].items() if key != "kind"}
    if "vector" in output:
        output["vector"] = [
            float(value.lexeme if isinstance(value, WireDecimal) else value)
            for value in output["vector"]
        ]
    if "sample_rate_hz" in output:
        output["sample_rate_hz"] = wire_integer_to_int(output["sample_rate_hz"])
    for wire_key, key in (("audio_base64", "audio"), ("image_base64", "image")):
        if wire_key in output:
            output[key] = _decode_media(output.pop(wire_key))
    return ModelResponse(reply["provider"], reply["model"], reply["units"], output_type(**output))


def embed(text: str, *, origin: Optional[Origin] = None, max_units: Optional[int] = None) -> ModelResponse[Embedding]:
    return _run_model({"kind": "embed", "text": text}, origin, max_units, Embedding)


def tts(text: str, *, origin: Optional[Origin] = None, max_units: Optional[int] = None) -> ModelResponse[Speech]:
    return _run_model({"kind": "tts", "text": text}, origin, max_units, Speech)


def stt(
    audio: bytes, *, origin: Optional[Origin] = None, max_units: Optional[int] = None,
) -> ModelResponse[Transcription]:
    """Transcribe WAV, MP3 or Ogg audio bytes; Yu OS detects the format."""
    return _run_model(
        {"kind": "stt", "audio_base64": _encode_media(audio, "stt: audio")},
        origin, max_units, Transcription,
    )


def imagegen(prompt: str, *, origin: Optional[Origin] = None, max_units: Optional[int] = None) -> ModelResponse[Image]:
    return _run_model({"kind": "imagegen", "prompt": prompt}, origin, max_units, Image)


def decide(
    state: WireJsonValue, questions: Mapping[str, WireJsonValue], *,
    origin: Optional[Origin] = None, max_units: Optional[int] = None,
) -> ModelResponse[Decision]:
    return _run_model(
        {"kind": "decision", "state": state, "questions": dict(questions)},
        origin, max_units, Decision,
    )


def open_permission_settings() -> None:
    """Open Applications settings after a user action; this does not authorize AI."""
    _model_call("settings", timeout=_DEFAULT_TIMEOUT_S)


def _model_call(
    command: str, *args: str, data: bytes = b"", timeout: Optional[float] = None,
) -> Dict[str, Any]:
    try:
        proc = subprocess.run(
            [_yu_binary(), WIRE_ARG, "ai", command, *args],
            input=data, capture_output=True, check=False, timeout=timeout,
        )
        return decode_response(proc.stdout.decode("utf-8"), proc.returncode, f"yu ai {command}")
    except WireDenied as error:
        _raise_for_error(error.payload)
    except (WireUnavailable, OSError, UnicodeError, subprocess.TimeoutExpired) as error:
        raise AiUnavailable(str(error)) from error


def budget(app_id: Optional[str] = None) -> BudgetUsage:
    """Return usage for the current period; the CLI does not report a cap."""
    app = app_id or os.environ.get("YUOS_APP_ID")
    if not app:
        raise AiError("budget: app_id is required")
    cmd = [_yu_binary(), WIRE_ARG, "agent", "budget", "show", app]
    try:
        proc = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=False,
            timeout=_DEFAULT_TIMEOUT_S,
        )
    except subprocess.TimeoutExpired as exc:
        raise AiUnavailable(
            f"yu agent budget show timed out after {_DEFAULT_TIMEOUT_S}s"
        ) from exc
    text = (proc.stdout or "").strip()
    try:
        env = decode_response(text, proc.returncode, "yu agent budget show")
    except (WireDenied, WireUnavailable) as exc:
        raise AiUnavailable(str(exc)) from exc
    try:
        validate_budget_show(env)
    except WireDecodeError as exc:
        raise AiUnavailable(f"budget response decode failed: {exc}") from exc
    return BudgetUsage(
        app=env["app"],
        period=env["period"],
        units_used=wire_integer_to_int(env["units_used"]),
    )


# ---------------------------------------------------------------------------
# Internals
# ---------------------------------------------------------------------------


def _dispatch(
    *,
    modality: str,
    prompt: Optional[str],
    origin: str,
    max_units: Optional[int],
    app_id: Optional[str],
    system: Optional[str] = None,
    tools: Optional[List[str]] = None,
) -> AiResponse:
    """Build the stable ``yu ai chat`` command and parse its envelope."""
    app = app_id or os.environ.get("YUOS_APP_ID") or ("dev" if _dev_ai_url() is not None else None)
    if not app:
        raise AiDenied({"code": "PERMISSION_DENIED", "error": f"{modality}: App identity is required"})

    with tempfile.TemporaryDirectory(prefix="yuos-ai-") as private_dir:
        cmd = [
            _yu_binary(),
            WIRE_ARG,
            "ai",
            "chat",
            "--app",
            app,
            "--origin",
            origin,
        ]

        def add_private_file(flag: str, name: str, value: str) -> None:
            path = os.path.join(private_dir, name)
            fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(fd, "w", encoding="utf-8") as output:
                output.write(value)
                output.flush()
                os.fsync(output.fileno())
            cmd.extend([flag, path])

        if prompt is not None:
            add_private_file("--prompt-file", "prompt", prompt)
        if max_units is not None:
            cmd.extend(["--max-units", str(max_units)])
        if system is not None:
            add_private_file("--system-file", "system", system)
        if tools:
            cmd.extend(["--tools", ",".join(tools)])

        try:
            proc = subprocess.run(cmd, capture_output=True, text=True, check=False)
        except (OSError, UnicodeError) as exc:
            raise AiUnavailable(str(exc)) from exc
    payload_text = (proc.stdout or "").strip()
    try:
        envelope = decode_response(payload_text, proc.returncode, "yu ai chat")
    except WireDenied as exc:
        _raise_for_error(exc.payload)
    except WireUnavailable as exc:
        raise AiUnavailable(str(exc)) from exc

    return _parse_response(envelope)


def _parse_response(env: Any) -> AiResponse:
    try:
        validate_ai(env)
    except WireDecodeError as exc:
        raise AiUnavailable(f"ai response decode failed: {exc}") from exc
    usage = env["usage"]
    budget_blk = env["budget"]
    review = env["review"]
    raw_calls = env.get("tool_calls", [])
    parsed_calls: List[ProposedToolCall] = []
    for tc in raw_calls:
        parsed_calls.append(
            ProposedToolCall(
                id=tc["id"],
                name=tc["name"],
                input=tc["input"],
            )
        )
    return AiResponse(
        text=env["text"],
        model=env["model"],
        provider=env["provider"],
        verb=env["verb"],
        usage=Usage(
            input_tokens=wire_integer_to_int(usage["input_tokens"]),
            output_tokens=wire_integer_to_int(usage["output_tokens"]),
            units=wire_integer_to_int(usage["units"]),
        ),
        budget=Budget(
            period=budget_blk["period"],
            units_used=wire_integer_to_int(budget_blk["units_used"]),
            units_cap=wire_integer_to_int(budget_blk["units_cap"]),
        ),
        review=Review(
            safety=review["safety"],
            prompt_redacted=review["prompt_redacted"],
        ),
        tool_calls=parsed_calls,
        raw=dict(env),
        provider_state=env.get("provider_state"),
    )


def _raise_for_error(env: Mapping[str, Any]) -> NoReturn:
    code = str(env["code"])
    if code == "BUDGET_EXCEEDED":
        raise AiBudgetExceeded(env)
    if code == "SAFETY_VIOLATION":
        raise AiSafetyViolation(env)
    message = str(env["error"])
    if code == "KERNEL_UNAVAILABLE":
        raise AiUnavailable(message, env)
    if code == "INVALID_ARGS":
        raise AiInvalidArgs(message, env)
    if code in {"PERMISSION_DENIED", "UNKNOWN_APP", "UNKNOWN_VERB", "DENIED"}:
        raise AiDenied(env)
    raise AiError(message, env)


def _yu_binary() -> str:
    override = os.environ.get("YUOS_KERNEL_BIN")
    if override:
        return override
    found = shutil.which("yu")
    if found is None:
        raise AiUnavailable(
            "the `yu` binary is not on PATH; cannot reach the AI gate"
        )
    return found


def _dev_ai_url() -> Optional[str]:
    if os.environ.get("YUOS_SESSION", "").strip():
        return None
    url = os.environ.get("YUOS_DEV_AI_URL", "").strip()
    return url.rstrip("/") if url else None


def _dev_ai_model(kind: str = "chat") -> str:
    slot_vars = {
        "chat": ("YUOS_DEV_AI_CHAT_MODEL", "gpt-6-sol"),
        "embed": ("YUOS_DEV_AI_EMBED_MODEL", "text-embedding-3-small"),
        "tts": ("YUOS_DEV_AI_TTS_MODEL", "tts-1"),
        "stt": ("YUOS_DEV_AI_STT_MODEL", "whisper-1"),
        "imagegen": ("YUOS_DEV_AI_IMAGEGEN_MODEL", "dall-e-3"),
        "decision": ("YUOS_DEV_AI_DECIDE_MODEL", "gpt-6-sol"),
        "decide": ("YUOS_DEV_AI_DECIDE_MODEL", "gpt-6-sol"),
    }
    env_var, default_model = slot_vars.get(kind, ("YUOS_DEV_AI_CHAT_MODEL", "gpt-6-sol"))
    specific = os.environ.get(env_var, "").strip()
    if specific:
        return specific
    shared = os.environ.get("YUOS_DEV_AI_MODEL", "").strip()
    return shared or default_model


def _dev_ai_key() -> Optional[str]:
    key = os.environ.get("YUOS_DEV_AI_KEY", "").strip()
    return key or None


def _dev_post_bytes(path: str, payload: Mapping[str, Any]) -> bytes:
    """POST to the local dev server; model replies have no SDK deadline."""
    base_url = _dev_ai_url()
    if not base_url:
        raise AiUnavailable("local dev AI URL is not configured")
    headers = {"Content-Type": "application/json"}
    key = _dev_ai_key()
    if key:
        headers["Authorization"] = f"Bearer {key}"
    req = urllib.request.Request(
        f"{base_url}{path}",
        data=json.dumps(payload).encode("utf-8"),
        headers=headers,
        method="POST",
    )
    try:
        with urllib.request.urlopen(req) as resp:
            status = getattr(resp, "status", 200)
            if status < 200 or status >= 300:
                raise AiUnavailable(f"dev AI HTTP {status}")
            return resp.read()
    except (urllib.error.URLError, urllib.error.HTTPError, OSError, ValueError) as error:
        raise AiUnavailable(f"dev AI request failed: {error}") from error


def _dev_post_json(path: str, payload: Mapping[str, Any]) -> Dict[str, Any]:
    raw = _dev_post_bytes(path, payload)
    try:
        parsed = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise AiUnavailable(f"dev AI returned invalid JSON: {error}") from error
    if not isinstance(parsed, dict):
        raise AiUnavailable("dev AI response must be a JSON object")
    return parsed


def _dev_chat(request: AiChatRequest) -> AiResponse:
    model = _dev_ai_model()
    messages: List[Dict[str, Any]] = []
    if "system" in request and request["system"] is not None:
        messages.append({"role": "system", "content": request["system"]})
    images = request.get("images")
    if images:
        content_parts: List[Dict[str, Any]] = [{"type": "text", "text": request["prompt"]}]
        for b64 in images:
            content_parts.append({"type": "image_url", "image_url": {"url": f"data:image/png;base64,{b64}"}})
        messages.append({"role": "user", "content": content_parts})
    else:
        messages.append({"role": "user", "content": request["prompt"]})
    for turn in request.get("tool_turns") or ():
        tool_calls_out: List[Dict[str, Any]] = []
        for c in turn.get("calls") or ():
            raw_args = c.get("input", c.get("arguments", {}))
            args_str = raw_args if isinstance(raw_args, str) else encode_wire_json(raw_args)
            tool_calls_out.append({
                "id": c["id"],
                "type": "function",
                "function": {"name": c["name"], "arguments": args_str},
            })
        assistant_msg: Dict[str, Any] = {"role": "assistant", "tool_calls": tool_calls_out}
        if "text" in turn and turn["text"] is not None:
            assistant_msg["content"] = turn["text"]
        messages.append(assistant_msg)
        for r in turn.get("results") or ():
            call_id = r.get("id") or r.get("call_id")
            if "content" in r and isinstance(r["content"], str):
                res_content = r["content"]
            else:
                res_content = encode_wire_json(r.get("output", r.get("content")))
            messages.append({"role": "tool", "tool_call_id": call_id, "content": res_content})
    body: Dict[str, Any] = {"model": model, "messages": messages}
    if request.get("tools"):
        body["tools"] = [
            {
                "type": "function",
                "function": {
                    "name": t["name"],
                    "description": t["description"],
                    "parameters": t["parameters"],
                },
            }
            for t in request["tools"]
        ]
    if "max_units" in request and request["max_units"] is not None:
        body["max_tokens"] = wire_integer_to_int(request["max_units"])
    resp = _dev_post_json("/chat/completions", body)
    choices = resp.get("choices")
    msg: Dict[str, Any] = {}
    if isinstance(choices, list) and choices and isinstance(choices[0], dict):
        raw_msg = choices[0].get("message")
        if isinstance(raw_msg, dict):
            msg = raw_msg
    content = msg.get("content")
    text = content if isinstance(content, str) else ""
    parsed_tool_calls: List[Dict[str, Any]] = []
    raw_tool_calls = msg.get("tool_calls")
    if isinstance(raw_tool_calls, list):
        for idx, tc in enumerate(raw_tool_calls):
            if not isinstance(tc, dict):
                raise AiUnavailable("dev AI tool_call must be an object")
            fn = tc.get("function")
            if not isinstance(fn, dict) or not isinstance(fn.get("name"), str):
                raise AiUnavailable("dev AI tool_call missing function name")
            raw_arguments = fn.get("arguments", "{}")
            if isinstance(raw_arguments, str):
                try:
                    decoded_args = json.loads(raw_arguments) if raw_arguments.strip() else {}
                except json.JSONDecodeError as error:
                    raise AiUnavailable(f"dev AI tool_call arguments invalid JSON: {error}") from error
            else:
                decoded_args = raw_arguments
            tc_id = tc.get("id")
            parsed_tool_calls.append({
                "id": tc_id if isinstance(tc_id, str) and tc_id else f"call_{idx}",
                "name": fn["name"],
                "input": decoded_args,
            })
    usage_obj = resp.get("usage") if isinstance(resp.get("usage"), dict) else {}
    total_tokens = usage_obj.get("total_tokens", 1)
    if not isinstance(total_tokens, int) or isinstance(total_tokens, bool) or total_tokens < 0:
        total_tokens = 1
    prompt_tokens = usage_obj.get("prompt_tokens", 0)
    if not isinstance(prompt_tokens, int) or isinstance(prompt_tokens, bool) or prompt_tokens < 0:
        prompt_tokens = 0
    completion_tokens = usage_obj.get("completion_tokens", 0)
    if not isinstance(completion_tokens, int) or isinstance(completion_tokens, bool) or completion_tokens < 0:
        completion_tokens = 0
    resp_model = resp.get("model")
    actual_model = resp_model if isinstance(resp_model, str) and resp_model.strip() else model
    env: Dict[str, Any] = {
        "text": text,
        "model": actual_model,
        "provider": "dev",
        "verb": "ai.chat",
        "usage": {
            "input_tokens": prompt_tokens,
            "output_tokens": completion_tokens,
            "units": total_tokens,
        },
        "budget": {"period": "dev", "units_used": 0, "units_cap": 0},
        "review": {"safety": "strict", "prompt_redacted": False},
    }
    if parsed_tool_calls:
        env["tool_calls"] = parsed_tool_calls
    return _parse_response(env)


def _dev_chat_stream(request: AiChatRequest) -> Iterator[str]:
    base_url = _dev_ai_url()
    assert base_url is not None
    model = _dev_ai_model()
    messages: List[Dict[str, Any]] = []
    if "system" in request and request["system"] is not None:
        messages.append({"role": "system", "content": request["system"]})
    images = request.get("images")
    if images:
        content_parts: List[Dict[str, Any]] = [{"type": "text", "text": request["prompt"]}]
        for b64 in images:
            content_parts.append({"type": "image_url", "image_url": {"url": f"data:image/png;base64,{b64}"}})
        messages.append({"role": "user", "content": content_parts})
    else:
        messages.append({"role": "user", "content": request["prompt"]})
    body: Dict[str, Any] = {"model": model, "messages": messages, "stream": True}
    if "max_units" in request and request["max_units"] is not None:
        body["max_tokens"] = wire_integer_to_int(request["max_units"])
    headers = {"Content-Type": "application/json"}
    key = _dev_ai_key()
    if key:
        headers["Authorization"] = f"Bearer {key}"
    req = urllib.request.Request(
        f"{base_url}/chat/completions",
        data=json.dumps(body).encode("utf-8"),
        headers=headers,
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=_DEFAULT_TIMEOUT_S) as resp:
            if resp.headers.get_content_type() != "text/event-stream":
                raise AiUnavailable("dev AI endpoint did not return a streaming response")
            while raw := resp.readline(8 * 1024 * 1024 + 1):
                if len(raw) > 8 * 1024 * 1024:
                    raise AiUnavailable("dev AI stream line exceeds the transport limit")
                line = raw.decode("utf-8").strip()
                if not line.startswith("data:"):
                    continue
                data_str = line[5:].strip()
                if data_str == "[DONE]":
                    return
                if not data_str:
                    continue
                chunk_obj = json.loads(data_str)
                choices = chunk_obj.get("choices") if isinstance(chunk_obj, dict) else None
                if isinstance(choices, list) and choices and isinstance(choices[0], dict):
                    delta = choices[0].get("delta")
                    piece = delta.get("content") if isinstance(delta, dict) else None
                    if isinstance(piece, str) and piece:
                        yield piece
    except (urllib.error.URLError, urllib.error.HTTPError, OSError, ValueError) as error:
        raise AiUnavailable(f"dev AI stream failed: {error}") from error
    raise AiUnavailable("dev AI stream ended without a final event")


def _dev_infer(request: AiInferenceRequest) -> Dict[str, Any]:
    inp = request["input"]
    kind = inp["kind"]
    model = _dev_ai_model(kind)
    units = 1
    actual_model = model
    if kind == "embed":
        resp = _dev_post_json("/embeddings", {"model": model, "input": inp["text"]})
        data = resp.get("data")
        if not isinstance(data, list) or not data or not isinstance(data[0], dict):
            raise AiUnavailable("dev AI embeddings response missing data[0]")
        embedding = data[0].get("embedding")
        if not isinstance(embedding, list):
            raise AiUnavailable("dev AI embeddings response missing embedding vector")
        output: Dict[str, Any] = {"kind": "embed", "vector": embedding}
        if isinstance(resp.get("model"), str) and resp["model"].strip():
            actual_model = resp["model"]
        usage = resp.get("usage")
        if isinstance(usage, dict) and isinstance(usage.get("total_tokens"), int) and usage["total_tokens"] >= 1:
            units = usage["total_tokens"]
    elif kind == "tts":
        raw = _dev_post_bytes(
            "/audio/speech",
            {"model": model, "input": inp["text"], "voice": "alloy", "response_format": "wav"},
        )
        audio_b64: Optional[str] = None
        fmt = "wav"
        try:
            parsed = json.loads(raw.decode("utf-8"))
            if isinstance(parsed, dict) and isinstance(parsed.get("audio_base64"), str):
                audio_b64 = parsed["audio_base64"]
                if isinstance(parsed.get("format"), str):
                    fmt = parsed["format"]
        except (UnicodeDecodeError, json.JSONDecodeError):
            pass
        if audio_b64 is None:
            if not raw:
                raise AiUnavailable("dev AI speech response is empty")
            audio_b64 = b64encode(raw).decode("ascii")
        output = {"kind": "tts", "format": fmt, "audio_base64": audio_b64}
    elif kind == "stt":
        resp = _dev_post_json(
            "/audio/transcriptions",
            {"model": model, "audio_base64": inp["audio_base64"]},
        )
        text = resp.get("text")
        if not isinstance(text, str):
            raise AiUnavailable("dev AI transcription response missing text")
        output = {"kind": "stt", "text": text}
    elif kind == "imagegen":
        resp = _dev_post_json(
            "/images/generations",
            {"model": model, "prompt": inp["prompt"], "response_format": "b64_json"},
        )
        data = resp.get("data")
        if not isinstance(data, list) or not data or not isinstance(data[0], dict):
            raise AiUnavailable("dev AI imagegen response missing data[0]")
        b64_json = data[0].get("b64_json")
        if not isinstance(b64_json, str):
            raise AiUnavailable("dev AI imagegen response missing b64_json")
        output = {"kind": "imagegen", "format": "png", "image_base64": b64_json}
    elif kind in ("decision", "decide"):
        user_payload = encode_wire_json({"state": inp["state"], "questions": inp["questions"]})
        resp = _dev_post_json(
            "/chat/completions",
            {
                "model": model,
                "response_format": {"type": "json_object"},
                "messages": [
                    {"role": "system", "content": 'Return JSON {"answers": {...}} answering each question.'},
                    {"role": "user", "content": user_payload},
                ],
            },
        )
        choices = resp.get("choices")
        if not isinstance(choices, list) or not choices or not isinstance(choices[0], dict):
            raise AiUnavailable("dev AI decision response missing choices[0]")
        msg = choices[0].get("message")
        if not isinstance(msg, dict) or not isinstance(msg.get("content"), str):
            raise AiUnavailable("dev AI decision response missing message content")
        try:
            parsed_content = json.loads(msg["content"])
        except json.JSONDecodeError as error:
            raise AiUnavailable(f"dev AI decision content is not valid JSON: {error}") from error
        if not isinstance(parsed_content, dict) or not isinstance(parsed_content.get("answers"), dict):
            raise AiUnavailable("dev AI decision content missing answers object")
        output = {"kind": kind, "answers": parsed_content["answers"]}
    else:
        raise AiInvalidArgs(f"unsupported inference kind: {kind}")
    return {
        "provider": "dev",
        "model": actual_model,
        "units": units,
        "output": output,
    }
