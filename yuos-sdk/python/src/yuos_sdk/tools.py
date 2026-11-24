"""Explicit catalog operations, separate from model calls and MCP.

The kernel resolves ``yu ai tool`` under the App's own capabilities and audits
the operation. The catalog is not exposed to the model by ``ai.chat``.
"""

from __future__ import annotations

import os
import shutil
import subprocess
from dataclasses import dataclass, field
from typing import Any, Dict, List, Mapping, Optional

from ._transport import WIRE_ARG, WireDenied, WireUnavailable, decode_response
from .generated import (
    WireDecodeError,
    WireJsonValue,
    encode_wire_json,
    validate_tool,
    validate_tool_catalog,
)


# Subprocess timeout — covers every shell-out to the `yu` binary so a
# wedged child never blocks the calling app forever.
_DEFAULT_TIMEOUT_S = 60
_MISSING = object()


# ---------------------------------------------------------------------------
# Exceptions
# ---------------------------------------------------------------------------


class ToolError(Exception):
    """Base class for every error this module raises."""


class ToolUnavailable(ToolError):
    """The ``yu`` binary could not be invoked or returned garbage."""


class ToolDenied(ToolError):
    """A gate (capability / unknown tool / args shape) refused the call.

    The ``payload`` attribute holds the structured envelope
    ``yu ai tool`` returned (verb, scope, reason, hint, …) —
    suitable for forwarding back to the agent.
    """

    def __init__(self, payload: Mapping[str, Any]):
        self.payload = dict(payload)
        super().__init__(self.payload.get("error") or "Tool call denied")


# ---------------------------------------------------------------------------
# Response shape
# ---------------------------------------------------------------------------


@dataclass
class ToolResult:
    """The kernel-mediated result of one tool invocation.

    ``value`` is the JSON the catalog implementation produced. Its
    shape is documented per-tool in ``docs/app-ai-tool-catalog.md``.
    ``status`` is ``"ok"`` on success; the kernel always raises
    :class:`ToolDenied` instead of returning a non-ok status here.
    """

    name: str
    app_id: str = ""
    status: str = "ok"
    value: WireJsonValue = None
    raw: Dict[str, WireJsonValue] = field(default_factory=dict)


@dataclass
class CatalogEntry:
    """One row from ``yu ai tools``."""

    name: str
    summary: str
    verb: str
    stability: str = "experimental"
    args_schema: Optional[Dict[str, WireJsonValue]] = None
    returns_schema: Optional[Dict[str, WireJsonValue]] = None
    raw: Dict[str, WireJsonValue] = field(default_factory=dict)


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def call(
    name: str,
    args: Any = _MISSING,
    *,
    app_id: Optional[str] = None,
) -> ToolResult:
    """Invoke a catalog tool through the kernel.

    ``args`` may be any JSON value, including explicit ``None``, a scalar,
    or an array. Shells to ``yu ai tool <name> --app <id> --args <json>``. The
    kernel resolves ``name`` against the catalog, derives the caps
    verb + scope, runs ``caps::require``, executes the implementation,
    and writes an audit row.

    Raises :class:`ToolDenied` for anything the gate refused
    (unknown tool, missing capability, malformed args, …) and
    :class:`ToolUnavailable` for transport problems.
    """
    if not name or not isinstance(name, str):
        raise ToolError("call: name must be a non-empty string")
    app = app_id or os.environ.get("YUOS_APP_ID")
    if not app:
        raise ToolError(
            f"{name}: app_id is required (pass app_id= or set YUOS_APP_ID)"
        )

    args_payload = encode_wire_json({} if args is _MISSING else args)
    cmd = [
        _yu_binary(),
        WIRE_ARG,
        "ai",
        "tool",
        name,
        "--app",
        app,
        "--args",
        args_payload,
    ]

    proc = _run_with_timeout(cmd, f"yu ai tool {name}")
    text = (proc.stdout or "").strip()
    try:
        envelope = decode_response(text, proc.returncode, f"yu ai tool {name}")
    except WireDenied as exc:
        raise ToolDenied(exc.payload) from exc
    except WireUnavailable as exc:
        raise ToolUnavailable(str(exc)) from exc

    try:
        validate_tool(envelope)
    except WireDecodeError as exc:
        raise ToolUnavailable(f"tool result decode failed: {exc}") from exc
    return ToolResult(
        name=envelope["tool"],
        app_id=envelope["app_id"],
        status=envelope["status"],
        value=envelope["result"],
        raw=dict(envelope),
    )


def catalog() -> List[CatalogEntry]:
    """Return the live tool catalog as exposed by ``yu ai tools``.

    Apps shouldn't hard-code tool names at install time without
    consulting this list; the catalog evolves and a tool can be
    deprecated or renamed between releases.
    """
    cmd = [_yu_binary(), WIRE_ARG, "ai", "tools"]
    proc = _run_with_timeout(cmd, "yu ai tools")
    text = (proc.stdout or "").strip()
    try:
        envelope = decode_response(text, proc.returncode, "yu ai tools")
    except WireDenied as exc:
        raise ToolDenied(exc.payload) from exc
    except WireUnavailable as exc:
        raise ToolUnavailable(str(exc)) from exc

    try:
        validate_tool_catalog(envelope)
    except WireDecodeError as exc:
        raise ToolUnavailable(f"catalog decode failed: {exc}") from exc
    rows = envelope["tools"]

    out: List[CatalogEntry] = []
    for row in rows:
        out.append(
            CatalogEntry(
                name=row["name"],
                summary=row["summary"],
                verb=row["verb"],
                stability=row["stability"],
                args_schema=dict(row["args_schema"]),
                returns_schema=dict(row["returns_schema"]),
                raw=dict(row),
            )
        )
    return out


def for_chat(*names: str) -> List[str]:
    """Legacy name normalization helper. Do not pass names to model-only chat."""
    out: List[str] = []
    for n in names:
        if not isinstance(n, str):
            raise ToolError(f"for_chat: tool names must be strings, got {n!r}")
        s = n.strip()
        if s:
            out.append(s)
    return out


# ---------------------------------------------------------------------------
# Internals
# ---------------------------------------------------------------------------


def _run_with_timeout(cmd: List[str], label: str) -> subprocess.CompletedProcess:
    """Run a `yu` subprocess with the bridge's default timeout. Raise
    :class:`ToolUnavailable` (not ``TimeoutExpired``) on timeout so
    callers can treat hangs uniformly with other transport failures.
    """
    try:
        return subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=False,
            timeout=_DEFAULT_TIMEOUT_S,
        )
    except subprocess.TimeoutExpired as exc:
        raise ToolUnavailable(
            f"{label} timed out after {_DEFAULT_TIMEOUT_S}s"
        ) from exc


def _yu_binary() -> str:
    override = os.environ.get("YUOS_KERNEL_BIN")
    if override:
        return override
    found = shutil.which("yu")
    if found is None:
        raise ToolUnavailable(
            "the `yu` binary is not on PATH; cannot reach the AI gate"
        )
    return found
