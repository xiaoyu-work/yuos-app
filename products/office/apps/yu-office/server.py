"""Yu Office document tools over the public MCP SDK.

`yu-office.read` and `yu-office.replace_text` are the legacy XML tools kept for
compatibility. The `yu-office.writer.*` tools come from the shared Writer
catalog and handlers that the in-Office assistant also uses.

Every tool reaches its existing file only through `BrokeredFile`, the OS
brokered read/replace adapter. The App never opens, snapshots or replaces the
requested path itself, and there is no raw-path fallback.
"""

from copy import copy
import hashlib
import io
import os
from pathlib import Path, PurePosixPath
import sys
from zipfile import BadZipFile, ZipFile

from lxml import etree

from yuos_sdk.mcp import App, CallCancelled, current_context


def _writer_pythonpath():
    here = Path(__file__).resolve().parent
    for candidate in (here / "share" / "extensions" / "yu-writer-ai" / "pythonpath",
                      here.parents[1] / "writer" / "extension" / "pythonpath"):
        if (candidate / "yu_writer" / "tools.py").is_file():
            return str(candidate)
    raise ImportError("The Yu Writer tool package is missing.")


sys.path.insert(0, _writer_pythonpath())
from yu_writer import tools as writer_tools  # noqa: E402
from yu_writer.catalog import Catalog, ToolError  # noqa: E402
from yu_writer.headless import FileTarget, HeadlessOffice  # noqa: E402

PROGRAM_DIR = Path(__file__).resolve().parent / "program"
BROKER_BINARY = "/usr/local/bin/yu"

WORD = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"
OFFICE = "{urn:oasis:names:tc:opendocument:xmlns:office:1.0}"
TEXT = "{urn:oasis:names:tc:opendocument:xmlns:text:1.0}"
XML_SPACE = "{http://www.w3.org/XML/1998/namespace}space"
ODT_MIME = b"application/vnd.oasis.opendocument.text"
DOCUMENT_MEMBERS = {".docx": "word/document.xml", ".odt": "content.xml"}
MAX_PACKAGE = 32 * 1024 * 1024
MAX_EXPANDED = 128 * 1024 * 1024
MAX_BODY_XML = 8 * 1024 * 1024
MAX_TEXT = 64_000


def _broker():
    """Import the OS file broker at the managed adapter boundary only."""
    try:
        from yuos_sdk.files import read_file, replace_file
    except ImportError:
        raise ToolError("broker_unavailable",
                        "This Yu OS SDK has no brokered file service, so Yu Office cannot open or "
                        "save documents for the System Agent.") from None
    return read_file, replace_file


class BrokeredFile:
    """One exact, OS-brokered document within one authenticated MCP call.

    The OS establishes authority from the manifest's `brokered-file` scope;
    the App passes the literal absolute path and never opens it. Reads and the
    single replace use the same call's deadline and cancellation. An uncertain
    replace is reported, never retried.
    """

    def __init__(self, path, *, edit):
        if (not isinstance(path, str) or not path.startswith("/")
                or any(ch in path for ch in "*?[\0")):
            raise ToolError("invalid_target",
                            "Name one exact absolute DOCX or ODT path; wildcards are not accepted.")
        self.path, self.edit = path, edit
        self.suffix = PurePosixPath(path).suffix.lower()
        if self.suffix not in DOCUMENT_MEMBERS:
            raise ToolError("invalid_target", "Yu Office supports DOCX and ODT documents only.")
        self._data = self._revision = None
        self._committed = False

    @staticmethod
    def _call_options():
        context = current_context()
        return {"binary": BROKER_BINARY, "deadline_unix_ms": context.deadline_unix_ms,
                "check_cancelled": context.raise_if_cancelled}

    def read(self):
        if self._data is None:
            read_file, _ = _broker()
            try:
                snapshot = read_file(self.path, **self._call_options())
            except CallCancelled:
                raise
            except Exception as error:  # noqa: BLE001 - broker refusal, missing or non-regular file
                raise ToolError("read_refused", f"The file broker did not provide the document ({error}).") from error
            data = snapshot.content
            if not isinstance(data, bytes) or not isinstance(snapshot.revision, str):
                raise ToolError("broker_invalid", "The file broker returned an invalid snapshot.")
            if len(data) > MAX_PACKAGE:
                raise ToolError("document_too_large", "The document must be no larger than 32 MiB.")
            if self.edit:
                try:
                    with ZipFile(io.BytesIO(data)) as archive:
                        names = [name.lower() for name in archive.namelist()]
                except BadZipFile:
                    raise ToolError("unreadable_document", "The document is not a DOCX or ODT package.") from None
                if any(name.startswith("_xmlsignatures/") or (name.startswith("meta-inf/") and "signatures" in name)
                       for name in names):
                    raise ToolError("unsupported_content", "Yu Office does not change signed documents.")
            self._data, self._revision = data, snapshot.revision
        return self._data

    def commit(self, data):
        if not self.edit:
            raise ToolError("unauthorized_target", "This call may not change the document.")
        if self._revision is None or self._committed:
            raise ToolError("invalid_state", "A document is saved once, after it was read in the same call.")
        if len(data) > MAX_PACKAGE:
            raise ToolError("document_too_large", "The updated document exceeds 32 MiB.")
        _, replace_file = _broker()
        self._committed = True
        try:
            replace_file(self.path, self._revision, data, **self._call_options())
        except CallCancelled:
            raise
        except Exception as error:  # noqa: BLE001 - broker refusal or uncertain outcome
            raise ToolError("save_unconfirmed",
                            f"The save was not confirmed ({error}); inspect the document before retrying.") from error
        return {"saved": True, "path": self.path, "sha256": hashlib.sha256(data).hexdigest()}


def _body(archive, suffix, *, edit=False):
    members = archive.infolist()
    names = {member.filename for member in members}
    if (len(names) != len(members) or sum(member.file_size for member in members) > MAX_EXPANDED
            or any(member.flag_bits & 1 for member in members)):
        raise ValueError("Duplicate, oversized or encrypted Office package.")
    if edit and any(name.startswith("_xmlsignatures/") or
                    (name.startswith("meta-inf/") and "signatures" in name)
                    for name in map(str.lower, names)):
        raise ValueError("Yu Office does not change signed documents.")
    member = DOCUMENT_MEMBERS[suffix]
    if member not in names or archive.getinfo(member).file_size > MAX_BODY_XML:
        raise ValueError("The document body is missing or larger than 8 MiB.")
    parser = etree.XMLParser(resolve_entities=False, no_network=True)
    root = etree.fromstring(archive.read(member), parser)
    if root.getroottree().docinfo.doctype:
        raise ValueError("Document type declarations are not supported.")
    if suffix == ".docx" and root.tag == WORD + "document":
        body = root.find(WORD + "body")
    elif (suffix == ".odt" and root.tag == OFFICE + "document-content"
          and "mimetype" in names and archive.getinfo("mimetype").file_size == len(ODT_MIME)
          and archive.read("mimetype") == ODT_MIME):
        body = root.find(f"{OFFICE}body/{OFFICE}text")
    else:
        body = None
    if body is None:
        raise ValueError("The package does not contain a supported Writer document body.")
    return root, body


def _odt_parts(node):
    if node.text:
        yield node, "text", node.text
    for child in node:
        if child.tag in {TEXT + "span", TEXT + "a"}:
            yield from _odt_parts(child)
        elif child.tag == TEXT + "s":
            yield None, "", int(child.get(TEXT + "c", "1"))
        elif child.tag == TEXT + "tab":
            yield None, "", "\t"
        elif child.tag == TEXT + "line-break":
            yield None, "", "\n"
        if child.tail:
            yield child, "tail", child.tail


def _paragraphs(body, suffix):
    if suffix == ".odt":
        space_budget = MAX_BODY_XML
        for paragraph in body.iter(TEXT + "p", TEXT + "h"):
            parts = []
            for node, field, value in _odt_parts(paragraph):
                if isinstance(value, int):
                    if not 1 <= value <= space_budget:
                        raise ValueError("Invalid or oversized ODT whitespace.")
                    space_budget -= value
                    value = " " * value
                parts.append((node, field, value))
            yield parts
    else:
        for paragraph in body.iter(WORD + "p"):
            parts = []
            for node in paragraph.iter(WORD + "t", WORD + "tab", WORD + "br", WORD + "cr"):
                if next(node.iterancestors(WORD + "p")) is not paragraph:
                    continue
                if node.tag == WORD + "t":
                    parts.append((node, "text", node.text or ""))
                else:
                    parts.append((None, "", "\t" if node.tag == WORD + "tab" else "\n"))
            yield parts


def read(path: str) -> dict:
    source = BrokeredFile(path, edit=False)
    data, suffix = source.read(), source.suffix
    with ZipFile(io.BytesIO(data)) as archive:
        _, body = _body(archive, suffix)
        paragraphs = ["".join(value for _, _, value in parts) for parts in _paragraphs(body, suffix)]
    text = "\n".join(paragraphs)
    return {"path": path, "text": text[:MAX_TEXT], "truncated": len(text) > MAX_TEXT,
            "sha256": hashlib.sha256(data).hexdigest()}


def _replace_one(body, suffix, find, replacement):
    match = None
    for parts in _paragraphs(body, suffix):
        text = "".join(value for _, _, value in parts)
        offset = text.find(find)
        if offset < 0:
            continue
        if match is not None or text.find(find, offset + 1) >= 0:
            raise ValueError("The requested text occurs more than once.")
        for node, field, value in parts:
            if node is not None and 0 <= offset <= len(value) - len(find):
                match = (node, field, value[:offset] + replacement + value[offset + len(find):])
                break
            offset -= len(value)
        else:
            raise ValueError("The requested text spans separate formatting or structure.")
    if match is None:
        raise ValueError("The requested text was not found in the document body.")
    node, field, value = match
    setattr(node, field, value)
    if suffix == ".docx" and value != value.strip():
        node.set(XML_SPACE, "preserve")


def replace_text(path: str, expected_sha256: str, find: str, replacement: str) -> dict:
    if not find or find == replacement or any(ch in find or ch in replacement for ch in "\t\r\n"):
        raise ValueError("Provide nonempty find text and a different single-line replacement.")
    document_file = BrokeredFile(path, edit=True)
    data, suffix = document_file.read(), document_file.suffix
    if hashlib.sha256(data).hexdigest() != expected_sha256:
        raise ValueError("Document revision changed; read it again before editing.")

    updated = io.BytesIO()
    with ZipFile(io.BytesIO(data)) as source, ZipFile(updated, "w") as target:
        root, body = _body(source, suffix, edit=True)
        _replace_one(body, suffix, find, replacement)
        document = etree.tostring(root.getroottree(), encoding="UTF-8", xml_declaration=True)
        if len(document) > MAX_BODY_XML:
            raise ValueError("The updated document body exceeds 8 MiB.")
        members = source.infolist()
        body_member = DOCUMENT_MEMBERS[suffix]
        if sum(member.file_size for member in members if member.filename != body_member) + len(document) > MAX_EXPANDED:
            raise ValueError("The updated document expands beyond 128 MiB.")
        target.comment = source.comment
        for member in members:
            contents = document if member.filename == body_member else source.read(member)
            target.writestr(copy(member), contents)
    result = updated.getvalue()
    if len(result) > MAX_PACKAGE:
        raise ValueError("The updated document exceeds 32 MiB.")
    document_file.commit(result)
    return {"path": path, "replacements": 1, "sha256": hashlib.sha256(result).hexdigest()}


def writer_tool(catalog, name):
    """Bind one catalog tool to the shared handler with a brokered file target."""

    def handler(**arguments):
        catalog.validate(name, arguments, bound_target=False)
        source = BrokeredFile(arguments["path"], edit=name in writer_tools.WRITE_TOOLS)
        source.read()
        with HeadlessOffice(PROGRAM_DIR) as office:
            target = FileTarget(source, office, check_cancelled=current_context().raise_if_cancelled)
            result = writer_tools.call(catalog, name, arguments, target)
        result.setdefault("path", source.path)
        return result

    handler.__name__ = name.rsplit(".", 1)[-1]
    return handler


def create_app():
    manifest = os.environ.get("YUOS_APP_MANIFEST") or "app.json"
    app = App.from_manifest(manifest)
    app.tool("yu-office.read")(read)
    app.tool("yu-office.replace_text")(replace_text)
    catalog = writer_tools.check_catalog(Catalog.load(manifest))
    for name in catalog.tools:
        app.tool(name)(writer_tool(catalog, name))
    return app


if __name__ == "__main__":
    create_app().serve()
