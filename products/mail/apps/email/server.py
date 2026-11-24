"""Authenticated MCP binding for the unified Thunderbird Email + AI App."""

from functools import wraps
import json
import os
from pathlib import Path
import sys

_HERE = Path(__file__).resolve().parent
if str(_HERE) not in sys.path:
    sys.path.insert(0, str(_HERE))
_ROOT = _HERE.parents[3]
for _p in (_ROOT / "yuos-sdk/python/src", _ROOT / "shared/python"):
    if _p.is_dir() and str(_p) not in sys.path:
        sys.path.insert(0, str(_p))

from yuos_sdk.mcp import App, ToolResult, _operation_argv, _tool_result  # noqa: E402
import ai_helpers as mail  # noqa: E402


def _mcp_result(handler):
    """Encode business failures as MCP tool errors, preserving the signature."""
    @wraps(handler)
    def invoke(**arguments):
        result = handler(**arguments)
        if "error" in result:
            return ToolResult(
                content=[{"type": "text", "text": result["error"]}],
                is_error=True,
                structured_content=result,
            )
        return result
    return invoke


def create_app() -> App:
    manifest_path = Path(os.environ.get("YUOS_APP_MANIFEST") or (_HERE / "app.json"))
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    app = App.from_manifest(manifest_path)
    operations = manifest.get("operations", {})
    for op_name, op_decl in operations.items():
        if op_name == "native-host":
            continue
        tool_name = f"email.{op_name}"
        def _bind(_op_name=op_name, _op_decl=op_decl):
            def invoke(**arguments):
                from main import run
                return _tool_result(run(_op_name, _operation_argv(_op_decl, arguments)))
            return invoke
        app.tool(tool_name)(_bind())
    app.tool("email.summarize")(_mcp_result(mail.summarize))
    app.tool("email.smart_reply")(_mcp_result(mail.smart_reply))
    app.tool("email.smart_compose")(_mcp_result(mail.smart_compose))
    app.tool("email.translate")(_mcp_result(mail.translate))
    app.tool("email.triage")(_mcp_result(mail.triage))
    app.tool("email.chat")(_mcp_result(mail.chat))
    return app


if __name__ == "__main__":
    create_app().serve()

