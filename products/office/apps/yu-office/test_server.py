from pathlib import Path
from types import SimpleNamespace
from zipfile import ZipFile

import pytest

from test_support import load_local_module


server = load_local_module(Path(__file__).with_name("server.py"), "yu_office_server")


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

    calls = []

    def authorize(verb, *, path):
        calls.append((verb, path))
        if verb == "fs.write" and not allow_write:
            raise PermissionError("write denied")

    allow_write = False
    monkeypatch.setattr(server.policy, "require", authorize)
    monkeypatch.setattr(server, "current_context", lambda: SimpleNamespace(session_id="test-session"))
    monkeypatch.setattr(server.snapshot, "snapshot",
                        lambda path, operation, *, session_id: calls.append((operation, path, session_id)))

    original = path.read_bytes()
    result = server.read(str(path))
    assert result["text"] == "Draft report"
    assert calls == [("fs.read", str(path))]
    with pytest.raises(PermissionError, match="write denied"):
        server.replace_text(str(path), result["sha256"], "Draft", "Final")
    assert path.read_bytes() == original

    allow_write = True
    with pytest.raises(ValueError, match="spans separate formatting"):
        server.replace_text(str(path), result["sha256"], "Draft report", "Final report")
    assert path.read_bytes() == original
    updated = server.replace_text(str(path), result["sha256"], "Draft", "Final")
    assert calls[-3:] == [("fs.read", str(path)), ("fs.write", str(path)),
                          ("write", str(path), "test-session")]
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
