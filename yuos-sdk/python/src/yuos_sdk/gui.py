"""Desktop GUI bootstrap for Yu OS apps.

This is the GUI counterpart to :mod:`yuos_sdk.mcp` (agent tools) and
:mod:`yuos_sdk.ai` (model access). It does **not** wrap a UI toolkit:
a Yu OS desktop app draws its own window in whatever toolkit/language
it likes ("World A"). All this module does is hand the app the small
amount of kernel context it needs once it has been kernel-spawned, plus
the one privileged action a GUI commonly wants — summoning the system
agent overlay.

## How a GUI app is launched

When an app declares a ``desktop`` block in ``app.json``, ``yu app
install`` writes a launcher whose ``Exec`` is ``yu app <id> --gui``.
Clicking it routes the launch back through the kernel, which spawns the
app's entry with ``YUOS_APP_GUI=1`` and ``YUOS_APP_ID`` set. Routing the
launch through ``yu app`` (rather than exec-ing the binary) is what
makes identity, audit, and consent apply to the GUI exactly as they do
to the headless operation path.

## Author surface

Mark the function that opens your window with ``@desktop``::

    from yuos_sdk import gui
    from yuos_sdk.app import desktop

    @desktop
    def main():
        ctx = gui.context()
        start_my_window(ctx)        # your toolkit, your loop

``ctx`` exposes:

* ``ctx.app_id``  — the kernel-assigned app identity.
* ``ctx.files``   — file paths the launcher passed (``%F``), if any.
* ``ctx.ai``      — the :mod:`yuos_sdk.ai` module (gated model access).
* ``ctx.tools``   — the :mod:`yuos_sdk.tools` module (call other apps).
* ``ctx.open_agent_overlay(hint=...)`` — summon the "Ask Yu OS" overlay.

The app's own files, sockets, and rendering are World A: use plain
stdlib for them. The kernel only interposes when you reach for ``ai`` /
``tools`` or a declared capability verb.
"""

from __future__ import annotations

import json
import os
import select
import socket
import stat
import struct
import subprocess
import threading
import time
import urllib.parse
from dataclasses import dataclass, field
from typing import Any, List, Optional

from . import ai, tools
from . import generated
from .kernel import call_json_with_stdin_binary

ASK_YU_LAUNCHER = "/usr/local/bin/yuos-ask-launcher"
ASK_YU_PROTOCOL = 1
ASK_YU_REQUEST_LIMIT = 32 * 1024


def request_file_read(node: int) -> generated.GuiFileRead:
    """Propose this view's native metadata inode; pending is not read authority."""
    request = {"node": node}
    generated.validate_gui_file_read_request(request)
    return _selected_read_call("request", request)


def file_read_status(selection_id: str, node: int) -> generated.GuiFileRead:
    """Read Root's status for this same window and inode; never approves."""
    request = {"selection_id": selection_id, "node": node}
    generated.validate_gui_file_read_status_request(request)
    return _selected_read_call("status", request)


def _selected_read_call(action: str, request: dict[str, Any]) -> generated.GuiFileRead:
    value = _selected_file_call("__gui-file-read", action, request)
    generated.validate_gui_file_read(value)
    return value


def request_file_write(node: int) -> generated.GuiFileWrite:
    """Request independent protected write review; pending never grants read/write."""
    request = {"node": node}
    generated.validate_gui_file_write_request(request)
    return _selected_write_call("request", request)


def file_write_status(selection_id: str, node: int) -> generated.GuiFileWrite:
    """Query this same window and file's Root-owned write selection."""
    request = {"selection_id": selection_id, "node": node}
    generated.validate_gui_file_write_status_request(request)
    return _selected_write_call("status", request)


def reserve_file_save(selection_id: str, node: int) -> generated.GuiFileSave:
    """Reserve private scratch for this selected target; not sibling write authority."""
    request = {"selection_id": selection_id, "node": node}
    generated.validate_gui_file_save_reservation_request(request)
    value = _selected_file_call("__gui-file-write", "reserve", request)
    generated.validate_gui_file_save(value)
    return value


def _selected_write_call(action: str, request: dict[str, Any]) -> generated.GuiFileWrite:
    value = _selected_file_call("__gui-file-write", action, request)
    generated.validate_gui_file_write(value)
    return value


def _selected_file_call(bridge: str, action: str, request: dict[str, Any]) -> Any:
    return call_json_with_stdin_binary(
        os.environ.get("YUOS_KERNEL_BIN", "/usr/local/bin/yu"),
        [bridge, action, "--request-stdin"],
        generated.encode_wire_json(request).encode("utf-8"),
        deadline_unix_ms=time.time_ns() // 1_000_000 + 30_000,
    )

def request_file_create(parent_node: int, name: str) -> generated.GuiFileCreate:
    """Request one absent literal child; metadata and pending create nothing."""
    request = {"parent_node": parent_node, "name": name}
    generated.validate_gui_file_create_request(request)
    return _selected_create_call("request", request)


def file_create_status(selection_id: str, parent_node: int) -> generated.GuiFileCreate:
    """Only created reports Root's committed creation, never a read grant."""
    request = {"selection_id": selection_id, "parent_node": parent_node}
    generated.validate_gui_file_create_status_request(request)
    return _selected_create_call("status", request)


def reserve_file_create(selection_id: str, parent_node: int) -> generated.GuiFileCreateSlot:
    """Reserve private native scratch for a one-shot NOREPLACE finalization."""
    request = {"selection_id": selection_id, "parent_node": parent_node}
    generated.validate_gui_file_create_reservation_request(request)
    value = _selected_file_call("__gui-file-create", "reserve", request)
    generated.validate_gui_file_create_slot(value)
    return value


def _selected_create_call(action: str, request: dict[str, Any]) -> generated.GuiFileCreate:
    value = _selected_file_call("__gui-file-create", action, request)
    generated.validate_gui_file_create(value)
    return value


def is_gui_launch() -> bool:
    """Return ``True`` when the current invocation is a desktop GUI launch.

    The bridge sets ``YUOS_APP_GUI=1`` for the authenticated desktop launch.
    Command strings are not accepted as a substitute for host context.
    """
    return os.environ.get("YUOS_APP_GUI") == "1"


def owner_home() -> Optional[str]:
    """Return the authenticated owner's home directory (`YUOS_OWNER_HOME`) when set."""
    raw = os.environ.get("YUOS_OWNER_HOME")
    if not raw or not os.path.isabs(raw):
        return None
    return os.path.realpath(raw)


@dataclass
class GuiContext:
    """The kernel context handed to a desktop app at launch.

    Construct with :func:`context`; do not instantiate directly in app
    code unless you are writing a test.
    """

    app_id: str
    files: List[str] = field(default_factory=list)
    #: The gated model-access module (:mod:`yuos_sdk.ai`).
    ai: Any = ai
    #: The cross-app verb-call module (:mod:`yuos_sdk.tools`).
    tools: Any = tools

    def open_agent_overlay(self, hint: Optional[str] = None) -> None:
        """Summon the system "Ask Yu OS" agent overlay.

        This uses the fixed packaged launcher's authenticated Unix-socket
        protocol. Pass ``hint`` to ground the agent's first response
        in the app's current state (e.g. the open document) without
        polluting the visible chat transcript.

        Raises :class:`FileNotFoundError` if the overlay binary is not
        installed (e.g. a headless box with no desktop shell).
        """
        _validate_launcher()
        child = subprocess.Popen(  # noqa: S603 - fixed trusted absolute path
            [ASK_YU_LAUNCHER, "--protocol", "1"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        try:
            assert child.stdout is not None
            announced, _, _ = select.select([child.stdout], [], [], 5)
            announcement = child.stdout.readline(257) if announced else b""
            prefix = f"SOCKET {ASK_YU_PROTOCOL} @".encode()
            if (
                len(announcement) > 256
                or not announcement.startswith(prefix)
                or not announcement.endswith(b"\n")
            ):
                raise RuntimeError("invalid Ask Yu OS socket announcement")
            endpoint = announcement[len(prefix) : -1]
            if not endpoint:
                raise RuntimeError("empty Ask Yu OS socket endpoint")
            request = {"protocol": ASK_YU_PROTOCOL, "app": self.app_id, "hint": hint}
            payload = json.dumps(request, separators=(",", ":")).encode()
            if len(payload) > ASK_YU_REQUEST_LIMIT:
                raise ValueError("Ask Yu OS request exceeds the protocol limit")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as channel:
                channel.settimeout(5)
                channel.connect(b"\0" + endpoint)
                if _read_exact(channel, 8) != b"READY 1\n":
                    raise RuntimeError("unexpected Ask Yu OS launcher handshake")
                channel.sendall(struct.pack(">I", len(payload)) + payload)
                channel.shutdown(socket.SHUT_WR)
                if _read_exact(channel, 11) != b"ACCEPTED 1\n":
                    raise RuntimeError("unexpected Ask Yu OS acceptance response")
        except Exception:
            child.kill()
            child.wait()
            raise
        try:
            threading.Thread(
                target=child.wait, name="ask-yu-sdk-reaper", daemon=True
            ).start()
        except Exception:
            child.kill()
            child.wait()
            raise


def _read_exact(channel: socket.socket, length: int) -> bytes:
    chunks = bytearray()
    while len(chunks) < length:
        chunk = channel.recv(length - len(chunks))
        if not chunk:
            raise RuntimeError("Ask Yu OS launcher closed the socket")
        chunks.extend(chunk)
    return bytes(chunks)


def _validate_launcher() -> None:
    for path in ("/usr", "/usr/local", "/usr/local/bin"):
        info = os.lstat(path)
        if (
            stat.S_ISLNK(info.st_mode)
            or not stat.S_ISDIR(info.st_mode)
            or info.st_uid != 0
            or info.st_mode & 0o022
        ):
            raise PermissionError(f"untrusted Ask Yu OS launcher parent: {path}")
    info = os.lstat(ASK_YU_LAUNCHER)
    if (
        stat.S_ISLNK(info.st_mode)
        or not stat.S_ISREG(info.st_mode)
        or info.st_uid != 0
        or not info.st_mode & 0o111
        or info.st_mode & 0o022
    ):
        raise PermissionError("untrusted Ask Yu OS launcher")


def _run_picker(
    mode: str,
    title: str,
    dev_env: str,
    name: Optional[str] = None,
    file_filter: Optional[str] = None,
) -> Optional[str]:
    if not os.environ.get("YUOS_SESSION"):
        dev_val = os.environ.get(dev_env)
        if dev_val is not None:
            return dev_val or None
    request_id = os.urandom(16).hex()
    payload: dict[str, Any] = {
        "request_id": request_id,
        "title": title,
        "modal": True,
    }
    if mode != "file":
        payload["mode"] = mode
    if name is not None:
        payload["name"] = name
    if file_filter is not None:
        payload["filter"] = file_filter
    _selected_file_call("__gui-file-picker", "open", payload)
    while True:
        res = _selected_file_call("__gui-file-picker", "status", {"request_id": request_id})
        status = res.get("status") if isinstance(res, dict) else None
        if status == "pending":
            time.sleep(0.1)
            continue
        if status == "cleanup_pending":
            raise RuntimeError("OS file picker cleanup is unconfirmed; do not replay")
        if status == "selected":
            uri = str(res.get("uri") or "")
            if uri.startswith("file://"):
                return urllib.parse.unquote(urllib.parse.urlparse(uri).path)
            return uri
        if status == "cancelled":
            return None
        error = res.get("error") if isinstance(res, dict) else None
        raise RuntimeError(str(error or "file picker failed"))


def open_file(title: str = "Open file", *, file_filter: Optional[str] = None) -> Optional[str]:
    """Open the system file picker and return the selected file path, or ``None`` if cancelled."""
    return _run_picker("file", title, "YUOS_DEV_OPEN_FILE", file_filter=file_filter)


def save_file(
    default_name: str,
    title: str = "Save file",
    *,
    file_filter: Optional[str] = None,
) -> Optional[str]:
    """Open the system Save dialog and return the chosen path, or ``None`` if cancelled."""
    if not isinstance(default_name, str) or not default_name.strip():
        raise ValueError("save_file: default_name must be a non-empty filename")
    return _run_picker("save", title, "YUOS_DEV_SAVE_FILE", name=default_name, file_filter=file_filter)


def select_folder(title: str = "Select folder") -> Optional[str]:
    """Open the system folder picker and return the selected directory path, or ``None`` if cancelled."""
    return _run_picker("folder", title, "YUOS_DEV_SELECT_FOLDER")


def context(files: Optional[List[str]] = None) -> GuiContext:
    """Build the :class:`GuiContext` for the current GUI launch.

    ``app_id`` is read from ``YUOS_APP_ID`` (set by the kernel when it
    spawns the GUI). ``files`` defaults to the launcher's file arguments,
    decoded from ``YUOS_ARGS_JSON`` when not supplied explicitly.
    """
    app_id = os.environ.get("YUOS_APP_ID")
    if not app_id:
        raise ValueError("YUOS_APP_ID is required for a GUI launch")
    if files is None:
        files = _files_from_env()
    if not isinstance(files, list) or any(not isinstance(item, str) for item in files):
        raise ValueError("GUI files must be a list of strings")
    return GuiContext(app_id=app_id, files=list(files))


def _files_from_env() -> List[str]:
    raw = os.environ.get("YUOS_ARGS_JSON")
    if not raw:
        return []
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError as error:
        raise ValueError("YUOS_ARGS_JSON must be valid JSON") from error
    if not isinstance(parsed, list) or any(not isinstance(item, str) for item in parsed):
        raise ValueError("YUOS_ARGS_JSON must be an array of strings")
    return parsed
