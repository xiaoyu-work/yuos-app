"""Root-brokered access to one explicitly authorized existing App MCP file."""

from __future__ import annotations

import base64
import binascii
import hashlib
import json
import re
import time
import unicodedata
from collections.abc import Callable
from contextlib import suppress
from dataclasses import dataclass
from typing import Any

from .kernel import KernelDenied, KernelUnavailable, call_json_with_stdin_binary

MAX_FILE_BYTES = 32 * 1024 * 1024
_CHUNK_BYTES = 512 * 1024
_TOKEN = re.compile(r"[0-9a-f]{32}")
_SHA256 = re.compile(r"sha256:[0-9a-f]{64}")
__all__ = ["FileSnapshot", "MAX_FILE_BYTES", "read_file", "replace_file"]


@dataclass(frozen=True)
class FileSnapshot:
    """A complete binary snapshot and an opaque revision for this invocation."""

    content: bytes
    revision: str


def _validate_path(path: str) -> None:
    if (
        not isinstance(path, str)
        or not path.startswith("/")
        or len(path.encode("utf-8")) > 4096
        or any(part in ("", ".", "..") for part in path.split("/")[1:])
        or any(unicodedata.category(ch) == "Cc" or ch in "*?[]{}<>|;&$`\\" for ch in path)
    ):
        raise ValueError("brokered file requires an absolute literal file path")


def _call(
    binary: str,
    deadline: int,
    cancel: Callable[[], None] | None,
    path: str,
    *,
    commit: bool = False,
    **fields: Any,
) -> dict[str, Any]:
    return call_json_with_stdin_binary(
        binary,
        ["__app_file", "commit" if commit else "transfer"],
        json.dumps({"path": path, **fields}, ensure_ascii=False).encode("utf-8"),
        deadline_unix_ms=deadline,
        check_cancelled=cancel,
    )


def _token(result: dict[str, Any], name: str) -> str:
    token = result.get(name)
    if not isinstance(token, str) or not _TOKEN.fullmatch(token):
        raise KernelUnavailable("broker returned an invalid file transfer token")
    return token


def _sha256(data: bytes | bytearray) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def _abort(binary: str, deadline: int, path: str, token: str) -> None:
    # Do not mask the original failure. Root also retires the stage with
    # the grant when cancellation or a lost response makes abort impossible.
    deadline = min(deadline, time.time_ns() // 1_000_000 + 250)
    with suppress(KernelDenied, KernelUnavailable):
        _call(binary, deadline, None, path, action="abort", id=token)


def read_file(
    path: str,
    *,
    binary: str,
    deadline_unix_ms: int,
    check_cancelled: Callable[[], None] | None = None,
) -> FileSnapshot:
    """Read an existing file in a ``brokered-file`` MCP invocation.

    Pass the bound absolute path, installed kernel binary, and the MCP
    call's absolute deadline and cancellation check. There is no local
    filesystem fallback, and a partial read never returns bytes.
    """
    _validate_path(path)
    metadata = _call(binary, deadline_unix_ms, check_cancelled, path, action="read-start")
    revision = _token(metadata, "revision")
    complete = False
    try:
        size = metadata.get("size")
        digest = metadata.get("sha256")
        if (
            type(size) is not int
            or not 0 <= size <= MAX_FILE_BYTES
            or not isinstance(digest, str)
            or not _SHA256.fullmatch(digest)
        ):
            raise KernelUnavailable("broker returned invalid file snapshot metadata")
        data = bytearray()
        for offset in range(0, size, _CHUNK_BYTES):
            part = _call(
                binary, deadline_unix_ms, check_cancelled, path,
                action="read-chunk", id=revision, offset=offset,
            )
            encoded = part.get("content_base64")
            count = min(_CHUNK_BYTES, size - offset)
            if (
                type(part.get("offset")) is not int
                or part["offset"] != offset
                or not isinstance(encoded, str)
                or len(encoded) != ((count + 2) // 3) * 4
            ):
                raise KernelUnavailable("broker returned an invalid file chunk")
            try:
                chunk = base64.b64decode(encoded, validate=True)
            except (binascii.Error, ValueError) as error:
                raise KernelUnavailable("broker returned invalid binary data") from error
            if len(chunk) != count:
                raise KernelUnavailable("broker returned a partial file chunk")
            data.extend(chunk)
        if _sha256(data) != digest:
            raise KernelUnavailable("broker returned a file snapshot with the wrong digest")
        result = FileSnapshot(content=bytes(data), revision=revision)
        complete = True
        return result
    finally:
        if not complete:
            _abort(binary, deadline_unix_ms, path, revision)


def replace_file(
    path: str,
    revision: str,
    content: bytes,
    *,
    binary: str,
    deadline_unix_ms: int,
    check_cancelled: Callable[[], None] | None = None,
) -> None:
    """Replace the exact existing file read earlier in this invocation.

    The success response means Root checked the revision and accepted the
    save (which may be unchanged). It is not consent or undo. Never
    automatically retry a commit after a failed or missing response.
    """
    _validate_path(path)
    if not isinstance(revision, str) or not _TOKEN.fullmatch(revision):
        raise ValueError("brokered file requires its opaque read revision")
    if not isinstance(content, bytes) or len(content) > MAX_FILE_BYTES:
        raise ValueError("replacement must be bytes and at most 32 MiB")
    started = _call(
        binary, deadline_unix_ms, check_cancelled, path, action="write-start",
        revision=revision, size=len(content), sha256=_sha256(content),
    )
    transfer = _token(started, "id")
    committing = False
    try:
        for offset in range(0, len(content), _CHUNK_BYTES):
            part = content[offset:offset + _CHUNK_BYTES]
            accepted = _call(
                binary, deadline_unix_ms, check_cancelled, path,
                action="write-chunk", id=transfer, offset=offset,
                content_base64=base64.b64encode(part).decode("ascii"),
            )
            received = accepted.get("offset")
            if type(received) is not int or received != offset + len(part):
                raise KernelUnavailable("broker did not acknowledge the full file chunk")
        committing = True
        result = _call(binary, deadline_unix_ms, check_cancelled, path, commit=True, id=transfer)
        if result.get("committed") is not True or type(result.get("changed")) is not bool:
            raise KernelUnavailable("broker returned no conclusive file commit result")
    finally:
        if not committing:
            _abort(binary, deadline_unix_ms, path, transfer)
