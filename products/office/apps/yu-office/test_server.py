import inspect
from io import BytesIO
from pathlib import Path
import sys
import time
from types import SimpleNamespace
from zipfile import ZipFile

import pytest

from test_support import load_local_module


server = load_local_module(Path(__file__).with_name("server.py"), "yu_office_server")
FakeBroker = load_local_module(
    Path(__file__).parents[2] / "writer" / "fake_broker.py", "yu_office_fake_broker").FakeBroker


def package(suffix, document, **members):
    data = BytesIO()
    with ZipFile(data, "w") as archive:
        if suffix == ".odt":
            archive.writestr("mimetype", server.ODT_MIME)
        archive.writestr(server.DOCUMENT_MEMBERS[suffix], document)
        for name, contents in members.items():
            archive.writestr(name.replace("__", "/"), contents)
    return data.getvalue()


@pytest.mark.parametrize("suffix", [".docx", ".odt"])
def test_read_and_replace_one_document_text_run(monkeypatch, suffix):
    if suffix == ".docx":
        document = (
            f'<w:document xmlns:w="{server.WORD[1:-1]}"><w:body>'
            '<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Draft</w:t></w:r>'
            '<w:r><w:t xml:space="preserve"> report</w:t></w:r></w:p>'
            '</w:body></w:document>'
        )
    else:
        document = (
            f'<o:document-content xmlns:o="{server.OFFICE[1:-1]}" xmlns:t="{server.TEXT[1:-1]}">'
            '<o:body><o:text><t:p><t:span t:style-name="Bold">Draft</t:span> report</t:p>'
            '</o:text></o:body></o:document-content>'
        )
    # The brokered path does not exist locally: the adapter can only use the broker.
    path = f"/brokered/owner/note{suffix}"
    assert not Path(path).parent.exists()
    broker = FakeBroker(deadline_unix_ms=1234).install(monkeypatch, server)
    broker.files[path] = original = package(suffix, document, **{"asset.bin": b"unrelated document bytes"})
    broker.deny.add("replace")

    result = server.read(path)
    assert result["text"] == "Draft report"
    assert broker.calls == [("read", path)]
    with pytest.raises(server.ToolError, match="save_unconfirmed"):
        server.replace_text(path, result["sha256"], "Draft", "Final")
    assert broker.files[path] == original and broker.calls.count(("replace", path)) == 1

    broker.deny.clear()
    with pytest.raises(ValueError, match="spans separate formatting"):
        server.replace_text(path, result["sha256"], "Draft report", "Final report")
    assert broker.files[path] == original
    broker.calls.clear()
    updated = server.replace_text(path, result["sha256"], "Draft", "Final")
    assert broker.calls == [("read", path), ("replace", path)]
    assert updated["replacements"] == 1
    read_back = server.read(path)
    assert read_back["text"] == "Final report"
    assert updated["sha256"] == read_back["sha256"]
    with ZipFile(BytesIO(broker.files[path])) as archive:
        assert archive.read("asset.bin") == b"unrelated document bytes"
        body_xml = archive.read(server.DOCUMENT_MEMBERS[suffix]).split(b"\n", 1)[1]
        assert body_xml == document.replace(">Draft<", ">Final<").encode()
    with pytest.raises(ValueError, match="revision changed"):
        server.replace_text(path, result["sha256"], "Final", "Outdated")
    assert server.read(path) == read_back
    assert not Path(path).parent.exists()


def test_existing_files_are_reached_only_through_the_broker(tmp_path, monkeypatch):
    path = tmp_path / "note.odt"
    path.write_bytes(b"not read directly")

    # Without the SDK file broker: refuse, never read the raw path.
    monkeypatch.setitem(sys.modules, "yuos_sdk.files", None)
    monkeypatch.delattr(sys.modules["yuos_sdk"], "files", raising=False)
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
    # A file that exists locally but is not brokered is refused, not opened.
    with pytest.raises(server.ToolError, match="read_refused"):
        server.read(str(path))

    signed = "/brokered/owner/signed.odt"
    broker.files[signed] = package(".odt", "<x/>", **{"META-INF__documentsignatures.xml": "<signed/>"})
    with pytest.raises(server.ToolError, match="unsupported_content"):
        server.BrokeredFile(signed, edit=True).read()
    source = server.BrokeredFile(signed, edit=False)
    source.read()
    with pytest.raises(server.ToolError, match="unauthorized_target"):
        source.commit(b"x")
    assert ("replace", signed) not in broker.calls


def test_adapter_matches_the_sdk_broker_and_has_no_local_fallback(tmp_path, monkeypatch):
    files = pytest.importorskip("yuos_sdk.files", reason="needs an SDK with the brokered file service")
    read_signature = inspect.signature(files.read_file)
    replace_signature = inspect.signature(files.replace_file)
    assert list(read_signature.parameters)[:1] == ["path"]
    assert list(replace_signature.parameters)[:3] == ["path", "revision", "content"]
    for signature in (read_signature, replace_signature):
        for name in ("binary", "deadline_unix_ms", "check_cancelled"):
            assert signature.parameters[name].kind is inspect.Parameter.KEYWORD_ONLY

    # A valid local document with no reachable broker must fail, not be opened directly.
    path = tmp_path / "note.odt"
    path.write_bytes(package(".odt", f'<o:document-content xmlns:o="{server.OFFICE[1:-1]}"/>'))
    original = path.read_bytes()
    monkeypatch.setattr(server, "BROKER_BINARY", str(tmp_path / "missing-yu"))
    context = SimpleNamespace(deadline_unix_ms=int(time.time() * 1000) + 10_000,
                              raise_if_cancelled=lambda: None)
    monkeypatch.setattr(server, "current_context", lambda: context)
    for call in (lambda: server.read(str(path)),
                 lambda: server.replace_text(str(path), "0" * 64, "a", "b")):
        with pytest.raises(server.ToolError, match="read_refused"):
            call()
    assert path.read_bytes() == original and sorted(p.name for p in tmp_path.iterdir()) == ["note.odt"]
