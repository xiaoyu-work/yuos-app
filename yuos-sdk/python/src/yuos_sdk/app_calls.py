"""Controlled App→App calls through the Yu OS App Gateway.

The caller declares ``"app_calls": ["<app>/<tool>"]`` in its manifest and
the target lists the caller in that tool's ``app_access``. The OS derives
the caller identity from its verified package; ``assertions`` are the
caller's own statements, accepted only when the target tool names them.
"""

from __future__ import annotations

import json
import os
import time
from typing import Any, Callable

from .kernel import call_json_with_stdin_binary

MAX_REQUEST_BYTES = 256 * 1024


def call(
    app_id: str,
    tool: str,
    arguments: dict[str, Any] | None = None,
    *,
    assertions: dict[str, str] | None = None,
    deadline_unix_ms: int | None = None,
    check_cancelled: Callable[[], None] | None = None,
    binary: str | None = None,
) -> dict[str, Any]:
    request: dict[str, Any] = {"app_id": app_id, "tool": tool, "arguments": arguments or {}}
    if assertions:
        request["assertions"] = assertions
    data = json.dumps(request, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    if len(data) > MAX_REQUEST_BYTES:
        raise ValueError("App call request exceeds 256 KiB")
    return call_json_with_stdin_binary(
        binary if binary is not None else os.environ.get("YUOS_KERNEL_BIN", "/usr/local/bin/yu"),
        ["__app_call"],
        data,
        deadline_unix_ms=(
            time.time_ns() // 1_000_000 + 120_000 if deadline_unix_ms is None else deadline_unix_ms
        ),
        check_cancelled=check_cancelled,
    )
