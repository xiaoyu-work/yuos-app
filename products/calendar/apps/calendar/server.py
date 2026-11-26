import json
import os
from pathlib import Path

from yuos_sdk import policy
from yuos_sdk.mcp import App, ToolResult, current_context, _operation_argv, _tool_result
from main import InviteImportError, import_invite, run


def import_invite_tool(ics: str, response: str | None = None) -> ToolResult:
    context = current_context()
    context.raise_if_cancelled()
    if context.caller.kind != "app" or context.caller.id != "email":
        return ToolResult.error("invite_not_applicable: only Email may import invitations")
    source = context.assertions.get("source_mailbox")
    organizer = context.assertions.get("organizer")
    if not source or not organizer:
        return ToolResult.error("invite_not_applicable: source_mailbox and organizer assertions are required")
    try:
        result = import_invite(ics, response, source=source, verified_organizer=organizer)
    except InviteImportError as error:
        return ToolResult.error(f"{error.code}: {error}")
    except (policy.PermissionDenied, policy.PolicyUnavailable) as error:
        return ToolResult.error(str(error))
    return ToolResult.structured(result)


def create_app() -> App:
    manifest_path = Path(os.environ.get("YUOS_APP_MANIFEST") or Path(__file__).with_name("app.json"))
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    app = App.from_manifest(manifest_path)
    for name, operation in manifest["operations"].items():
        def bind(operation_name=name, operation_declaration=operation):
            def invoke(**arguments):
                return _tool_result(run(
                    operation_name,
                    _operation_argv(operation_declaration, arguments),
                ))
            return invoke
        app.tool(f"calendar.{name}")(bind())
    app.tool("calendar.import_invite")(import_invite_tool)
    return app


if __name__ == "__main__":
    create_app().serve()
