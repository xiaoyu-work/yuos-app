"""Writer tools against a real LibreOffice UNO document model.

Set YU_OFFICE_TEST_PROGRAM to a LibreOffice `program` directory whose pyuno
matches this Python (Yu Office's private runtime, or a distro stand-in).
"""

import os
from pathlib import Path
from types import SimpleNamespace

import pytest

from test_support import load_local_module

WRITER = Path(__file__).resolve().parent
MANIFEST = WRITER.parent / "apps" / "yu-office" / "app.json"
PROGRAM = Path(os.environ.get("YU_OFFICE_TEST_PROGRAM", "/usr/lib/libreoffice/program"))
pytestmark = pytest.mark.skipif(not (PROGRAM / "soffice.bin").is_file(),
                                reason="needs a LibreOffice program directory with pyuno")

server = load_local_module(MANIFEST.with_name("server.py"), "yu_office_writer_server")
from yu_writer import document as model, tools  # noqa: E402
from yu_writer.catalog import Catalog, ToolError  # noqa: E402
from yu_writer.headless import FILTERS, HeadlessOffice, _properties  # noqa: E402


def make_document(path):
    with HeadlessOffice(PROGRAM) as office:
        document = office.desktop.loadComponentFromURL(
            "private:factory/swriter", "_blank", 0, _properties(office.uno, Hidden=True))
        text = document.getText()
        cursor = text.createTextCursor()
        text.insertString(cursor, "Draft title", False)
        cursor.ParaStyleName = "Heading 1"
        for content in ("Plain body text.", None, "After table"):
            text.insertControlCharacter(cursor, 0, False)
            cursor.ParaStyleName = "Standard"
            if content is None:
                text.insertString(cursor, "Mixed ", False)
                cursor.CharWeight = 150.0
                text.insertString(cursor, "bold", False)
                cursor.CharWeight = 100.0
                text.insertControlCharacter(cursor, 0, False)
                table = document.createInstance("com.sun.star.text.TextTable")
                table.initialize(2, 2)
                text.insertTextContent(cursor, table, False)
                table.getCellByName("A1").setString("cell")
                cursor = text.createTextCursorByRange(text.getEnd())
            else:
                text.insertString(cursor, content, False)
        document.storeToURL(path.as_uri(), _properties(office.uno, FilterName=FILTERS[path.suffix]))
        document.close(True)


def by_text(state, text):
    return next(block for block in state["blocks"] if block.get("text") == text)


@pytest.fixture
def catalog():
    return Catalog.load(MANIFEST)


@pytest.fixture
def mcp(monkeypatch, catalog):
    calls = []
    monkeypatch.setattr(server, "PROGRAM_DIR", PROGRAM)
    monkeypatch.setattr(server.policy, "require", lambda verb, *, path: calls.append((verb, path)))
    monkeypatch.setattr(server, "current_context", lambda: SimpleNamespace(session_id="test-session"))
    monkeypatch.setattr(server.snapshot, "snapshot",
                        lambda path, operation, *, session_id: calls.append((operation, path)))
    return calls, lambda name, **arguments: server.writer_tool(catalog, name)(**arguments)


@pytest.mark.parametrize("suffix", [".docx", ".odt"])
def test_mcp_inspect_preview_apply_on_private_copy(tmp_path, mcp, suffix):
    calls, run = mcp
    path = tmp_path / f"report{suffix}"
    make_document(path)
    state = run(tools.INSPECT, path=str(path))
    kinds = [block["kind"] for block in state["blocks"]]
    assert kinds[:2] == ["heading", "paragraph"] and "table" in kinds and state["objects"]["tables"] == 1
    heading, plain = by_text(state, "Draft title"), by_text(state, "Plain body text.")
    mixed = by_text(state, "Mixed bold")
    table = next(block for block in state["blocks"] if block["kind"] == "table")
    assert heading["heading_level"] == 1 and mixed["mixed_formatting"] and not table["rewritable"]
    assert calls == [("fs.read", str(path))]
    original = path.read_bytes()
    revision = state["revision"]

    preview = run(tools.PREVIEW, path=str(path), revision=revision, action="rewrite",
                  paragraphs=[plain["index"]], texts=["Final body text."])
    assert preview["applied"] is False and preview["changes"][0]["before"] == "Plain body text."
    assert path.read_bytes() == original and ("fs.write", str(path)) not in calls

    refusals = [
        ({"action": "rewrite", "paragraphs": [mixed["index"]], "texts": ["x"]}, "mixed_formatting"),
        ({"action": "rewrite", "paragraphs": [table["index"]], "texts": ["x"]}, "unsupported_content"),
        ({"action": "rewrite", "paragraphs": [plain["index"]], "texts": ["a\nb"]}, "invalid_arguments"),
        ({"action": "format", "paragraphs": [plain["index"]], "paragraph_style": "No Such Style"}, "invalid_arguments"),
        ({"action": "format", "paragraphs": [plain["index"]], "bold": True, "revision": "0" * 64}, "stale_revision"),
    ]
    for arguments, code in refusals:
        with pytest.raises(ToolError, match=code):
            run(tools.APPLY, path=str(path), **{"revision": revision, **arguments})
    assert path.read_bytes() == original

    rewritten = run(tools.APPLY, path=str(path), revision=revision, action="rewrite",
                    paragraphs=[plain["index"], mixed["index"]], texts=["Final body text.", "Mixed flat"],
                    flatten_formatting=True)
    assert rewritten["saved"] and rewritten["warnings"] and ("write", str(path)) in calls
    state = run(tools.INSPECT, path=str(path))
    assert by_text(state, "Final body text.")["format"]["bold"] is False
    assert by_text(state, "Mixed flat")["mixed_formatting"] is False
    assert state["objects"]["tables"] == 1

    with pytest.raises(ToolError, match="stale_revision"):
        run(tools.APPLY, path=str(path), revision=revision, action="format",
            paragraphs=[heading["index"]], bold=True)
    formatted = run(tools.APPLY, path=str(path), revision=state["revision"], action="format",
                    paragraphs=[heading["index"]], alignment="center", italic=True, font_size=20)
    assert formatted["applied"]
    title = by_text(run(tools.INSPECT, path=str(path)), "Draft title")
    assert title["alignment"] == "center" and title["format"]["italic"] and title["format"]["font_size"] == 20
    assert title["heading_level"] == 1


def test_mcp_adapter_rejects_wildcards_missing_and_nonregular_targets(tmp_path, mcp):
    _, run = mcp
    (tmp_path / "folder.odt").mkdir()
    for path, code in ((tmp_path / "*.odt", "invalid_target"), (tmp_path / "missing.odt", "missing_target"),
                       (tmp_path / "folder.odt", "invalid_target"), (tmp_path / "notes.txt", "invalid_target")):
        with pytest.raises(ToolError, match=code):
            run(tools.INSPECT, path=str(path))


def test_live_document_apply_is_one_undo_step_and_failures_roll_back(tmp_path, catalog, monkeypatch):
    path = tmp_path / "live.odt"
    make_document(path)
    with HeadlessOffice(PROGRAM) as office:
        document = office.load(path.read_bytes(), ".odt")
        target = tools.LiveDocumentTarget(document)
        with pytest.raises(ToolError, match="unauthorized_target"):
            tools.call(catalog, tools.INSPECT, {"path": str(path)}, target)
        state = tools.call(catalog, tools.INSPECT, {}, target)
        plain = by_text(state, "Plain body text.")
        arguments = {"revision": state["revision"], "action": "format", "paragraphs": [plain["index"]],
                     "bold": True, "alignment": "right"}
        result = tools.call(catalog, tools.APPLY, arguments, target)
        assert result["saved"] is False and result["undo"]["steps"] == 1 and document.isModified()
        assert by_text(tools.call(catalog, tools.INSPECT, {}, target), "Plain body text.")["format"]["bold"]
        manager = document.getUndoManager()
        assert manager.getCurrentUndoActionTitle() == tools.UNDO_TITLE
        manager.undo()
        assert model.revision(document) == state["revision"]

        original = model.apply_changes

        def partial_then_fail(*args):
            original(*args)
            raise RuntimeError("simulated failure")

        monkeypatch.setattr(model, "apply_changes", partial_then_fail)
        with pytest.raises(RuntimeError):
            tools.call(catalog, tools.APPLY, arguments, target)
        assert model.revision(document) == state["revision"]
