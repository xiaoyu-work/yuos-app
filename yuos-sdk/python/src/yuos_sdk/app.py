"""Mark the functions Yu OS starts for an App's command line and desktop.

Declare operations once under ``operations`` in ``app.json``; command-line and
desktop launches share that declaration. If an App only implements
``@command`` or only implements ``@desktop``, the SDK returns a default
unavailable-surface response when the other surface is launched::

    from yuos_sdk.app import command, desktop

    @command                   # yu app <id> summarize "text"
    def summarize(args):
        return {"summary": args[0]}

    @desktop                   # App launcher or yu app <id> --gui
    def main():
        ...                    # Open your window with any GUI toolkit.
"""

from __future__ import annotations

import importlib.util
import inspect
import json
import os
import sys
import time
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, TypeVar, Union, overload

from .kernel import call_json_with_stdin_binary

__all__ = [
    "PermissionDenied",
    "check_permission",
    "clipboard_read",
    "clipboard_write",
    "command",
    "data_dir",
    "desktop",
    "launch_app",
    "log",
    "notify",
    "open_url",
    "require_permission",
    "run",
    "secret",
]

Handler = TypeVar("Handler", bound=Callable[..., Any])

_commands: Dict[str, Callable[[List[str]], Any]] = {}
_desktop_commands: Dict[str, Callable[..., Any]] = {}
_desktop: Optional[Callable[..., Any]] = None
_dispatched: bool = False
_dev_clipboard: str = ""


class PermissionDenied(PermissionError):
    """Raised when the Yu OS policy gate denies a requested capability."""

    def __init__(self, decision: Dict[str, Any]) -> None:
        self.denial = dict(decision)
        super().__init__(self.denial.get("summary") or f"permission denied: {self.denial.get('verb', '')}")


def _resolve_yu_binary() -> str:
    import shutil

    return (
        os.environ.get("YUOS_KERNEL_BIN")
        or os.environ.get("YUOS_BIN")
        or shutil.which("yu")
        or "/usr/local/bin/yu"
    )


@overload
def command(name: Handler) -> Handler: ...


@overload
def command(name: str) -> Callable[[Handler], Handler]: ...


def _is_same_module_duplicate(existing: Optional[Callable[..., Any]], new_fn: Callable[..., Any]) -> bool:
    if existing is None:
        return False
    if existing is new_fn:
        return True
    mod_a = getattr(existing, "__module__", None)
    mod_b = getattr(new_fn, "__module__", None)
    file_a = getattr(getattr(existing, "__code__", None), "co_filename", None)
    file_b = getattr(getattr(new_fn, "__code__", None), "co_filename", None)
    name_a = getattr(existing, "__name__", None)
    name_b = getattr(new_fn, "__name__", None)
    if file_a and file_b and file_a != file_b:
        return False
    if file_a and file_a == file_b and name_a == name_b:
        # Module reload in tests: same file and same function name.
        return False
    return mod_a == mod_b


def command(name: Union[Handler, str]) -> Any:
    """Run the decorated function for ``yu app <id> <name>``.

    Can be used as ``@command`` (uses the function name) or
    ``@command("name")``. It receives the command's validated arguments and
    returns a JSON value.
    """
    if callable(name):
        fn = name
        cmd_name = getattr(fn, "__name__", "")
        if not cmd_name:
            raise ValueError("command function must have a name")
        if _is_same_module_duplicate(_commands.get(cmd_name), fn):
            raise ValueError(f"command {cmd_name!r} already has a function")
        _commands[cmd_name] = fn
        return fn

    if not isinstance(name, str) or not name:
        raise ValueError("command name must be a nonempty string")

    def register(handler: Handler) -> Handler:
        if _is_same_module_duplicate(_commands.get(name), handler):
            raise ValueError(f"command {name!r} already has a function")
        _commands[name] = handler
        return handler

    return register


@overload
def desktop(handler: Handler) -> Handler: ...


@overload
def desktop(handler: str) -> Callable[[Handler], Handler]: ...


def desktop(handler: Union[Handler, str]) -> Any:
    """Run the decorated function to open the App's desktop window."""
    global _desktop
    if isinstance(handler, str):
        name = handler
        if not name:
            raise ValueError("desktop operation name must be a nonempty string")

        def register(fn: Handler) -> Handler:
            if name in _desktop_commands:
                raise ValueError(f"desktop operation {name!r} already has a function")
            _desktop_commands[name] = fn
            return fn

        return register

    if _desktop is not None:
        raise ValueError("the desktop window already has a function")
    _desktop = handler
    return handler


def data_dir() -> Path:
    """Return the App's private data directory, creating it if needed."""
    env_dir = os.environ.get("YUOS_DATA_DIR")
    if env_dir:
        path = Path(env_dir)
    else:
        xdg = os.environ.get("XDG_DATA_HOME")
        base = Path(xdg) if xdg else (Path.home() / ".local" / "share")
        app_id = os.environ.get("YUOS_APP_ID") or "dev"
        path = base / "yuos-dev" / "apps" / app_id
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    return path


def notify(summary: str, body: str = "") -> Dict[str, Any]:
    """Post a system notification from this App (`ui.notify`)."""
    if not isinstance(summary, str) or not summary.strip():
        raise ValueError("notify: summary must be a non-empty string")
    if not os.environ.get("YUOS_SESSION"):
        return {"status": "dev", "summary": summary, "body": body}
    payload = {
        "action": "post",
        "summary": summary,
        "body": body,
        "app_name": os.environ.get("YUOS_APP_ID") or "",
        "icon": "",
        "expire_ms": 0,
        "transient": False,
    }
    deadline = time.time_ns() // 1_000_000 + 10_000
    return call_json_with_stdin_binary(
        os.environ.get("YUOS_KERNEL_BIN", "/usr/local/bin/yu"),
        ["__notifications", "request", "--request-stdin", "--deadline", str(deadline)],
        json.dumps(payload).encode("utf-8"),
        deadline_unix_ms=deadline,
    )


def clipboard_read() -> str:
    """Read text from the system clipboard (`clipboard.read`)."""
    if not os.environ.get("YUOS_SESSION"):
        return os.environ.get("YUOS_DEV_CLIPBOARD", _dev_clipboard)
    deadline = time.time_ns() // 1_000_000 + 10_000
    res = call_json_with_stdin_binary(
        os.environ.get("YUOS_KERNEL_BIN", "/usr/local/bin/yu"),
        ["__clipboard", "read"],
        b"",
        deadline_unix_ms=deadline,
    )
    if isinstance(res, dict):
        if isinstance(res.get("text"), str):
            return res["text"]
        if isinstance(res.get("content_base64"), str):
            from base64 import b64decode

            return b64decode(res["content_base64"]).decode("utf-8", errors="replace")
    return ""


def clipboard_write(text: str) -> None:
    """Write text to the system clipboard (`clipboard.write`)."""
    global _dev_clipboard
    if not isinstance(text, str):
        raise ValueError("clipboard_write: text must be a string")
    if not os.environ.get("YUOS_SESSION"):
        _dev_clipboard = text
        return
    scratch = data_dir() / ".clipboard-tmp"
    scratch.write_text(text, encoding="utf-8")
    try:
        deadline = time.time_ns() // 1_000_000 + 10_000
        call_json_with_stdin_binary(
            os.environ.get("YUOS_KERNEL_BIN", "/usr/local/bin/yu"),
            ["__clipboard", "write", "--source", str(scratch), "--confirm"],
            b"",
            deadline_unix_ms=deadline,
        )
    finally:
        try:
            scratch.unlink()
        except OSError:
            pass


def secret(name: str, namespace: str = "default") -> str:
    """Load a secret from the Yu OS credential store (`secret.read`)."""
    if not isinstance(name, str) or not name.strip():
        raise ValueError("secret: name must be a non-empty string")
    if not isinstance(namespace, str) or not namespace.strip():
        raise ValueError("secret: namespace must be a non-empty string")
    env_key = "YUOS_DEV_SECRET_" + "".join(
        ch if ch.isalnum() else "_" for ch in name.strip().upper()
    )
    if os.environ.get("YUOS_DEV_MODE") == "1" or env_key in os.environ or "YUOS_DEV_SECRET" in os.environ:
        return os.environ.get(env_key, os.environ.get("YUOS_DEV_SECRET", ""))
    deadline = time.time_ns() // 1_000_000 + 10_000
    res = call_json_with_stdin_binary(
        _resolve_yu_binary(),
        ["credential", "load", name.strip(), "--namespace", namespace.strip()],
        b"",
        deadline_unix_ms=deadline,
    )
    if isinstance(res, dict) and isinstance(res.get("value"), str):
        return res["value"]
    raise RuntimeError(f"credential {namespace}/{name} has no value")


def check_permission(
    verb: str,
    *,
    path: Optional[str] = None,
    host: Optional[str] = None,
    name: Optional[str] = None,
    self_ref: Optional[str] = None,
    wild: bool = False,
) -> Dict[str, Any]:
    """Check whether the current session may exercise ``verb`` on a dynamic scope."""
    if not isinstance(verb, str) or not verb.strip():
        raise ValueError("check_permission: verb must be a non-empty string")
    supplied = [
        ("--path", path),
        ("--host", host),
        ("--name", name),
        ("--self", self_ref),
    ]
    supplied = [(flag, val) for flag, val in supplied if val is not None]
    if wild:
        supplied.append(("--wild", None))
    if len(supplied) > 1:
        raise ValueError(
            "check_permission accepts at most one of path / host / name / self_ref / wild"
        )
    if os.environ.get("YUOS_DEV_MODE") == "1":
        return {"decision": "allow", "verb": verb, "granted": True}
    argv = ["__policy", "check", verb]
    if supplied:
        flag, val = supplied[0]
        argv.append(flag)
        if val is not None:
            argv.append(val)
    deadline = time.time_ns() // 1_000_000 + 10_000
    res = call_json_with_stdin_binary(
        _resolve_yu_binary(),
        argv,
        b"",
        deadline_unix_ms=deadline,
    )
    if not isinstance(res, dict) or "decision" not in res:
        raise RuntimeError("policy check response omitted `decision`")
    return res


def require_permission(
    verb: str,
    *,
    path: Optional[str] = None,
    host: Optional[str] = None,
    name: Optional[str] = None,
    self_ref: Optional[str] = None,
    wild: bool = False,
) -> None:
    """Require that the current session may exercise ``verb`` on a dynamic scope."""
    decision = check_permission(
        verb,
        path=path,
        host=host,
        name=name,
        self_ref=self_ref,
        wild=wild,
    )
    if decision.get("decision") != "allow":
        raise PermissionDenied(decision)


def launch_app(app_id: str, uri: Optional[str] = None) -> Dict[str, Any]:
    """Launch a desktop application by freedesktop AppID (`desktop.launch`)."""
    if not isinstance(app_id, str) or not app_id.strip():
        raise ValueError("launch_app: app_id must be a non-empty string")
    argv = ["__desktop", "launch", "--app-id", app_id.strip()]
    if uri is not None:
        if not isinstance(uri, str) or not uri.strip() or "\x00" in uri:
            raise ValueError("launch_app: uri must be a non-empty string")
        argv.extend(["--uri", uri.strip()])
    if not os.environ.get("YUOS_SESSION"):
        return {"launched": True, "app_id": app_id.strip(), "uri": uri, "status": "dev"}
    deadline = time.time_ns() // 1_000_000 + 10_000
    res = call_json_with_stdin_binary(
        os.environ.get("YUOS_KERNEL_BIN", "/usr/local/bin/yu"),
        argv,
        b"",
        deadline_unix_ms=deadline,
    )
    return res if isinstance(res, dict) else {"launched": True, "app_id": app_id.strip()}


def open_url(url: str) -> Dict[str, Any]:
    """Open an http(s) or mailto URL in the desktop browser (`desktop.launch`)."""
    if not isinstance(url, str) or not url.strip().startswith(("http://", "https://", "mailto:")):
        raise ValueError("open_url: url must start with http://, https://, or mailto:")
    return launch_app("com.yuos.Browser", uri=url.strip())


def log(message: str, level: str = "info") -> Dict[str, Any]:
    """Record a structured App log entry (`data.log.write`) to `data_dir()/logs/app.jsonl` and stderr."""
    if not isinstance(message, str) or not message.strip():
        raise ValueError("log: message must be a non-empty string")
    if level not in ("debug", "info", "warn", "error"):
        raise ValueError("log: level must be debug, info, warn, or error")
    if os.environ.get("YUOS_SESSION"):
        require_permission("data.log.write", wild=True)
    from datetime import datetime, timezone

    entry: Dict[str, Any] = {
        "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "app": os.environ.get("YUOS_APP_ID") or "dev",
        "level": level,
        "message": message,
    }
    log_dir = data_dir() / "logs"
    log_dir.mkdir(parents=True, exist_ok=True)
    line = json.dumps(entry, ensure_ascii=False)
    with open(log_dir / "app.jsonl", "a", encoding="utf-8") as handle:
        handle.write(line + "\n")
    sys.stderr.write(f"[{level.upper()}] {message}\n")
    return entry


def _call_desktop_fn(fn: Callable[..., Any], resolved_args: List[str]) -> Any:
    try:
        sig = inspect.signature(fn)
        params = [
            p
            for p in sig.parameters.values()
            if p.kind in (inspect.Parameter.POSITIONAL_ONLY, inspect.Parameter.POSITIONAL_OR_KEYWORD)
            or p.kind == inspect.Parameter.VAR_POSITIONAL
        ]
        if not params:
            return fn()
    except (TypeError, ValueError):
        pass
    return fn(resolved_args)


def _default_unavailable(surface: str, command_name: Optional[str] = None) -> Dict[str, Any]:
    if surface == "desktop":
        payload: Dict[str, Any] = {
            "available": False,
            "surface": "desktop",
            "message": "This feature is only available on the command line.",
        }
    else:
        payload = {
            "available": False,
            "surface": "cli",
            "message": "This feature is only available in the desktop app.",
        }
    if command_name:
        payload["command"] = command_name
    return payload


def _parse_env_args() -> Optional[List[str]]:
    raw = os.environ.get("YUOS_ARGS_JSON")
    if raw is None or raw == "":
        return None
    try:
        parsed = json.loads(raw)
    except ValueError as error:
        raise RuntimeError(f"YUOS_ARGS_JSON must be an array of strings: {error}") from error
    if not isinstance(parsed, list) or any(not isinstance(item, str) for item in parsed):
        raise RuntimeError("YUOS_ARGS_JSON must be an array of strings")
    return list(parsed)


def _run_desktop(resolved_args: List[str], cli_invoked: bool) -> Any:
    if resolved_args and resolved_args[0] in _desktop_commands:
        result = _call_desktop_fn(_desktop_commands[resolved_args[0]], resolved_args[1:])
    elif _desktop is not None:
        result = _call_desktop_fn(_desktop, resolved_args)
    elif len(_desktop_commands) == 1:
        only_fn = next(iter(_desktop_commands.values()))
        result = _call_desktop_fn(only_fn, resolved_args)
    elif _commands:
        result = _default_unavailable("desktop")
    else:
        raise RuntimeError("main.py has no @desktop function")
    if cli_invoked and result is not None:
        print(json.dumps(result))
    return result


def _serve_mcp_from_commands() -> None:
    from .mcp import App, ToolResult

    manifest_path = Path(os.environ.get("YUOS_APP_MANIFEST") or "app.json")
    mcp_app = App.from_manifest(manifest_path)
    raw_manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    app_id = raw_manifest.get("id", "")
    tools = raw_manifest.get("mcp", {}).get("tools", [])
    for tool_decl in tools:
        tool_name = tool_decl.get("name", "")
        short_name = tool_name[len(app_id) + 1 :] if tool_name.startswith(f"{app_id}.") else tool_name
        handler = _commands.get(short_name) or _commands.get(tool_name)
        if handler is None:
            continue

        def _make_tool_fn(fn: Callable[..., Any], decl: Dict[str, Any]) -> Callable[..., Any]:
            def _invoke(**kwargs: Any) -> Any:
                argv: List[str] = []
                for arg_decl in decl.get("args", []):
                    aname = arg_decl["name"]
                    if aname not in kwargs or kwargs[aname] is None:
                        continue
                    val = kwargs[aname]
                    binding = arg_decl.get("binding") or ("flag" if arg_decl.get("kind") == "bool" else "positional")
                    flag_str = f"--{aname.replace('_', '-')}"
                    if arg_decl.get("kind") == "bool":
                        if val is True:
                            argv.append(flag_str)
                        elif val is False:
                            argv.append(f"{flag_str}=false")
                    elif binding == "flag":
                        if isinstance(val, list):
                            for item in val:
                                argv.extend([flag_str, str(item)])
                        else:
                            argv.extend([flag_str, str(val)])
                    else:
                        if isinstance(val, list):
                            argv.extend(str(item) for item in val)
                        else:
                            argv.append(str(val))
                res = fn(argv)
                if isinstance(res, dict) and ("error" in res or res.get("ok") is False):
                    tr = ToolResult.structured(res)
                    return ToolResult(content=tr.content, is_error=True, structured_content=res)
                return res

            return _invoke

        mcp_app.tool(tool_name)(_make_tool_fn(handler, tool_decl))
    mcp_app.serve()


def run(name: Optional[str] = None, args: Any = None) -> Any:
    """Call the function for this launch. Yu OS calls this for ``main.py``."""
    global _dispatched
    _dispatched = True
    cli_invoked = name is None
    if cli_invoked and os.environ.get("YUOS_MCP_SERVER") == "1" and _commands:
        _serve_mcp_from_commands()
        return None
    env_args = _parse_env_args()

    def _coerce_args(raw: Any, fallback: List[str]) -> Any:
        if raw is None:
            return fallback
        if isinstance(raw, dict):
            return raw
        return list(raw)

    if os.environ.get("YUOS_APP_GUI") == "1":
        resolved_args = _coerce_args(args, env_args if env_args is not None else [])
        return _run_desktop(resolved_args if isinstance(resolved_args, list) else [], cli_invoked)

    if name is None:
        env_cmd = os.environ.get("YUOS_COMMAND")
        if env_cmd:
            name = env_cmd
            resolved_args = _coerce_args(args, env_args if env_args is not None else [])
        else:
            cli_args = list(args) if args is not None and not isinstance(args, dict) else list(sys.argv[1:])
            if cli_args and cli_args[0] == "--gui":
                return _run_desktop(cli_args[1:], cli_invoked)
            name = cli_args[0] if cli_args else ""
            resolved_args = env_args if env_args is not None else cli_args[1:]
    else:
        resolved_args = _coerce_args(args, env_args if env_args is not None else [])

    handler = _commands.get(name)
    if handler is None:
        if _desktop is not None or _desktop_commands:
            result = _default_unavailable("cli", name or None)
            if cli_invoked:
                print(json.dumps(result))
            return result
        raise RuntimeError(f"main.py has no @command({name!r}) function")
    result = handler(resolved_args)
    if cli_invoked and result is not None:
        print(json.dumps(result))
    return result


def _main(argv: Optional[List[str]] = None) -> None:
    cli = list(sys.argv[1:] if argv is None else argv)
    if not cli:
        raise SystemExit("usage: python3 -m yuos_sdk.app <path-to-main.py> [<command>|--gui] [args...]")
    script_path = cli[0]
    sys.modules.setdefault("yuos_sdk.app", sys.modules[__name__])
    app_mod = sys.modules["yuos_sdk.app"]
    app_mod._dispatched = False
    app_mod._commands.clear()
    app_mod._desktop_commands.clear()
    app_mod._desktop = None
    old_argv = sys.argv
    sys.argv = [script_path, *cli[1:]]
    try:
        spec = importlib.util.spec_from_file_location("__yuos_app_main__", script_path)
        if spec is None or spec.loader is None:
            raise RuntimeError(f"cannot load {script_path!r}")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        if not getattr(app_mod, "_dispatched", False):
            entry = getattr(module, "run", None)
            if callable(entry) and entry is not app_mod.run:
                env_args = _parse_env_args()
                if os.environ.get("YUOS_APP_GUI") == "1" or (
                    cli[1:2] == ["--gui"] and not os.environ.get("YUOS_COMMAND")
                ):
                    gui_args = env_args if env_args is not None else cli[2:]
                    entry("--gui", gui_args)
                else:
                    cmd_name = os.environ.get("YUOS_COMMAND") or (cli[1] if len(cli) > 1 else "")
                    cmd_args = env_args if env_args is not None else cli[2:]
                    result = entry(cmd_name, cmd_args)
                    if result is not None:
                        print(json.dumps(result))
            else:
                app_mod.run()
    finally:
        sys.argv = old_argv


if __name__ == "__main__":
    _main()
