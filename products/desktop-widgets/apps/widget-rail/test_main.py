import json
from pathlib import Path

APP = Path(__file__).resolve().parent


def test_identity_and_independent_observer_grants():
    manifest = json.loads((APP / "app.json").read_text())
    assert manifest["id"] == "widget-rail"
    assert manifest["runtime"] == "binary"
    assert manifest["entry"] == manifest["mcp"]["entry"] == "bin/yuos-applet-widget-rail"
    assert manifest["mcp"]["transport"] == "stdio"
    assert manifest["mcp"]["lifecycle"] == "lazy"
    assert list(manifest["operations"]) == ["open"]
    needs = manifest["operations"]["open"]["needs"]
    assert [(need["verb"], need["scope"]) for need in needs] == [
        ("sys.observe", {"kind": "fixed", "scope": {"kind": "name", "value": "resources"}}),
    ]
    assert manifest["desktop"]["single_instance"] is True
    assert manifest["desktop"]["panel_applet"] is True
    assert manifest["icon"] == "view-grid-symbolic"
    assert manifest["desktop"]["exec"] == "open"


def test_system_tool_has_observer_need_without_panel_authority():
    manifest = json.loads((APP / "app.json").read_text())
    [tool] = manifest["mcp"]["tools"]
    assert tool["name"] == "widget-rail.system"
    assert tool["needs"] == manifest["operations"]["open"]["needs"]
    assert tool["args"] == []
    assert [effect["kind"] for effect in tool["effects"]] == ["read"]
    assert [(need["verb"], need["scope"]) for need in manifest["desktop"]["needs"]] == [
        ("sys.observe", {"kind": "fixed", "scope": {"kind": "name", "value": "resources"}}),
        ("ui.panel", {"kind": "fixed", "scope": {"kind": "wild"}}),
    ]
