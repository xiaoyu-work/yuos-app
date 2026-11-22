from pathlib import Path
import sys
from zipfile import ZipFile

import pytest

from test_support import load_local_module


server = load_local_module(Path(__file__).with_name("server.py"), "yu_office_server")
FakeBroker = load_local_module(
    Path(__file__).parents[2] / "writer" / "fake_broker.py", "yu_office_fake_broker").FakeBroker


@pytest.mark.parametrize("suffix", [".docx", ".odt"])
def test_read_and_replace_one_document_text_run(tmp_path, monkeypatch, suffix):
    path = tmp_path / f"note{suffix}"
    with ZipFile(path, "w") as archive:
        if suffix == ".docx":
            document = (
                f'<w:document xmlns:w="{server.WORD[1:-1]}"><w:body>'
                '<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Draft</w:t></w:r>'
                '<w:r><w:t xml:space="preserve"> report</w:t></w:r></w:p>'
                '</w:body></w:document>'
            )
        else:
            archive.writestr("mimetype", server.ODT_MIME)
            document = (
                f'<o:document-content xmlns:o="{server.OFFICE[1:-1]}" xmlns:t="{server.TEXT[1:-1]}">'
                '<o:body><o:text><t:p><t:span t:style-name="Bold">Draft</t:span> report</t:p>'
                '</o:text></o:body></o:document-content>'
            )
        archive.writestr(server.DOCUMENT_MEMBERS[suffix], document)
        archive.writestr("asset.bin", b"unrelated document bytes")

    broker = FakeBroker(deadline_unix_ms=1234).install(monkeypatch, server)
    broker.deny.add("replace")

    original = path.read_bytes()
    result = server.read(str(path))
    assert result["text"] == "Draft report"
    assert broker.calls == [("read", str(path))]
    with pytest.raises(server.ToolError, match="save_unconfirmed"):
        server.replace_text(str(path), result["sha256"], "Draft", "Final")
    assert path.read_bytes() == original and broker.calls.count(("replace", str(path))) == 1

    broker.deny.clear()
    with pytest.raises(ValueError, match="spans separate formatting"):
        server.replace_text(str(path), result["sha256"], "Draft report", "Final report")
    assert path.read_bytes() == original
    broker.calls.clear()
    updated = server.replace_text(str(path), result["sha256"], "Draft", "Final")
    assert broker.calls == [("read", str(path)), ("replace", str(path))]
    assert updated["replacements"] == 1
    read_back = server.read(str(path))
    assert read_back["text"] == "Final report"
    assert updated["sha256"] == read_back["sha256"]
    with ZipFile(path) as archive:
        assert archive.read("asset.bin") == b"unrelated document bytes"
        body_xml = archive.read(server.DOCUMENT_MEMBERS[suffix]).split(b"\n", 1)[1]
        assert body_xml == document.replace(">Draft<", ">Final<").encode()
    with pytest.raises(ValueError, match="revision changed"):
        server.replace_text(str(path), result["sha256"], "Final", "Outdated")
    assert server.read(str(path)) == read_back


def test_existing_files_are_reached_only_through_the_broker(tmp_path, monkeypatch):
    path = tmp_path / "note.odt"
    path.write_bytes(b"not read directly")

    # The 0.1.0 SDK has no file broker: refuse, never read the raw path.
    monkeypatch.setitem(sys.modules, "yuos_sdk.files", None)
    for call in (lambda: server.read(str(path)),
                 lambda: server.replace_text(str(path), "0" * 64, "a", "b")):
        with pytest.raises(server.ToolError, match="broker_unavailable"):
            call()
    assert path.read_bytes() == b"not read directly"

    broker = FakeBroker().install(monkeypatch, server)
    for target in ("note.odt", str(tmp_path / "*.odt"), str(tmp_path / "notes.txt")):
        with pytest.raises(server.ToolError, match="invalid_target"):
            server.read(target)
    assert broker.calls == []

    with ZipFile(path, "w") as archive:
        archive.writestr("META-INF/documentsignatures.xml", "<signed/>")
    with pytest.raises(server.ToolError, match="unsupported_content"):
        server.BrokeredFile(str(path), edit=True).read()
    source = server.BrokeredFile(str(path), edit=False)
    source.read()
    with pytest.raises(server.ToolError, match="unauthorized_target"):
        source.commit(b"x")
    assert ("replace", str(path)) not in broker.calls
