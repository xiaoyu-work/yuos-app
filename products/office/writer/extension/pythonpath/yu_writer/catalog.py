"""The one Writer tool catalog, read from the signed Yu Office manifest.

Both AI surfaces use this catalog: the System Agent receives these tools over
MCP and the in-Office assistant validates model proposals against them. The
`path` argument names the document target. MCP binds it to an OS-authorized
file; the GUI binds the live Writer document and rejects a model-supplied path.
"""

import copy
import json
import math
import os
from pathlib import Path

APP_ID = "yu-office"
PREFIX = "yu-office.writer."
TARGET_ARG = "path"
MAX_MANIFEST_BYTES = 1024 * 1024
JSON_TYPES = {
    "path": "string", "name": "string", "text": "string",
    "integer": "integer", "number": "number", "bool": "boolean",
}


class ToolError(ValueError):
    """A user-visible refusal with a stable machine-readable code."""

    def __init__(self, code, message):
        super().__init__(message)
        self.code = code

    def __str__(self):
        return f"{self.code}: {self.args[0]}"


def manifest_path(module_file=__file__):
    """Find the signed package manifest without trusting a model or document."""
    configured = os.environ.get("YUOS_APP_MANIFEST")
    if configured:
        return Path(configured)
    here = Path(module_file).resolve()
    for parent in here.parents:
        resources = parent / "resources"
        if (parent / "app.json").is_file() and resources.is_dir() and here.is_relative_to(resources):
            return parent / "app.json"
    raise ToolError("catalog_unavailable", "The signed Yu Office manifest was not found.")


class Tool:
    def __init__(self, raw):
        self.name = raw["name"]
        summary = raw.get("summary")
        self.summary = summary.get("en") if isinstance(summary, dict) else summary
        self.args = [dict(arg) for arg in raw.get("args", [])]
        self.by_name = {arg["name"]: arg for arg in self.args}
        if TARGET_ARG not in self.by_name or self.by_name[TARGET_ARG].get("kind") != "path":
            raise ToolError("catalog_invalid", f"{self.name} must declare its path target.")

    def input_schema(self, *, bound_target=False):
        properties, required = {}, []
        for arg in self.args:
            if bound_target and arg["name"] == TARGET_ARG:
                continue
            item = {"type": JSON_TYPES[arg["kind"]]}
            if arg.get("choices"):
                item["enum"] = list(arg["choices"])
            prop = {"type": "array", "items": item} if arg.get("repeatable") else item
            label = arg.get("label")
            if isinstance(label, dict) and label.get("en"):
                prop["description"] = label["en"]
            if "default" in arg:
                prop["default"] = arg["default"]
            properties[arg["name"]] = prop
            if arg.get("required"):
                required.append(arg["name"])
        schema = {"type": "object", "properties": properties, "additionalProperties": False}
        if required:
            schema["required"] = required
        return schema


class Catalog:
    def __init__(self, manifest):
        if not isinstance(manifest, dict) or manifest.get("id") != APP_ID:
            raise ToolError("catalog_invalid", "The manifest is not the Yu Office App.")
        tools = (manifest.get("mcp") or {}).get("tools") or []
        self.tools = {raw["name"]: Tool(raw) for raw in tools if raw.get("name", "").startswith(PREFIX)}
        if not self.tools:
            raise ToolError("catalog_invalid", "The manifest declares no Writer tools.")
        self.app_id = manifest["id"]

    @classmethod
    def load(cls, path=None):
        path = Path(path) if path is not None else manifest_path()
        with path.open("rb") as stream:
            raw = stream.read(MAX_MANIFEST_BYTES + 1)
        if len(raw) > MAX_MANIFEST_BYTES:
            raise ToolError("catalog_invalid", "The Yu Office manifest is too large.")
        return cls(json.loads(raw.decode("utf-8")))

    def tool(self, name):
        tool = self.tools.get(name)
        if tool is None:
            raise ToolError("unknown_tool", f"{name!r} is not a Yu Office Writer tool.")
        return tool

    def choices(self, name, arg):
        return list(self.tool(name).by_name[arg].get("choices", []))

    def validate(self, name, arguments, *, bound_target):
        """Apply the manifest contract; a bound target forbids a supplied path."""
        tool = self.tool(name)
        if not isinstance(arguments, dict):
            raise ToolError("invalid_arguments", "Tool arguments must be an object.")
        if bound_target and TARGET_ARG in arguments:
            raise ToolError("unauthorized_target", "This surface edits only its bound document; omit `path`.")
        extras = sorted(set(arguments) - set(tool.by_name))
        if extras:
            raise ToolError("invalid_arguments", f"Unknown argument `{extras[0]}`.")
        resolved = {}
        for arg in tool.args:
            key = arg["name"]
            if bound_target and key == TARGET_ARG:
                continue
            if key not in arguments:
                if "default" in arg:
                    resolved[key] = copy.deepcopy(arg["default"])
                elif arg.get("required"):
                    raise ToolError("invalid_arguments", f"Missing required argument `{key}`.")
                continue
            value = arguments[key]
            if arg.get("repeatable"):
                if not isinstance(value, list):
                    raise ToolError("invalid_arguments", f"`{key}` must be an array.")
                resolved[key] = [_scalar(key, arg, item) for item in value]
            else:
                resolved[key] = _scalar(key, arg, value)
        return resolved


def _scalar(key, arg, value):
    kind = arg["kind"]
    if kind in {"path", "name", "text"}:
        ok = isinstance(value, str)
    elif kind == "bool":
        ok = isinstance(value, bool)
    elif kind == "integer":
        ok = isinstance(value, int) and not isinstance(value, bool)
    else:
        ok = (isinstance(value, (int, float)) and not isinstance(value, bool)
              and math.isfinite(value))
    if not ok:
        raise ToolError("invalid_arguments", f"`{key}` must be a {JSON_TYPES[kind]}.")
    if arg.get("choices") and value not in arg["choices"]:
        raise ToolError("invalid_arguments", f"`{key}` is not one of its allowed values.")
    return value
