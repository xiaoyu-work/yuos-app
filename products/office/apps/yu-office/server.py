"""Yu Office document tools over the public MCP SDK.

`yu-office.read` and `yu-office.replace_text` are the legacy XML tools kept for
compatibility. The `yu-office.writer.*` tools come from the shared Writer
catalog and handlers that the in-Office assistant also uses.
"""

from copy import copy
import hashlib
import io
import os
from pathlib import Path
import stat
import sys
from zipfile import BadZipFile, ZipFile

from lxml import etree

from _shared.atomic import atomic_write_bytes
from _shared.paths import safe_realpath
from yuos_runtime import policy, snapshot
from yuos_sdk.mcp import App, current_context


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


def _revision(info):
    return (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns)


def _read_package(path, *, edit=False):
    path = safe_realpath(path)
    suffix = Path(path).suffix.lower()
    if suffix not in DOCUMENT_MEMBERS:
        raise ValueError("Yu Office supports DOCX and ODT documents only.")
    policy.require("fs.read", path=path)
    if edit:
        policy.require("fs.write", path=path)
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as stream:
        info = os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode) or info.st_size > MAX_PACKAGE:
            raise ValueError("The document must be a regular file no larger than 32 MiB.")
        data = stream.read(MAX_PACKAGE + 1)
        if len(data) > MAX_PACKAGE or _revision(os.fstat(stream.fileno())) != _revision(info):
            raise ValueError("The document changed while reading it; retry the read.")
    return path, suffix, data, info


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
    path, suffix, data, _ = _read_package(path)
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
    path, suffix, data, original = _read_package(path, edit=True)
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

    if _revision(os.stat(path, follow_symlinks=False)) != _revision(original):
        raise ValueError("Document revision changed; read it again before editing.")
    snapshot.snapshot(path, "write", session_id=current_context().session_id)
    if _revision(os.stat(path, follow_symlinks=False)) != _revision(original):
        raise ValueError("Document revision changed; read it again before editing.")
    atomic_write_bytes(path, result, mode=stat.S_IMODE(original.st_mode) & 0o777, strict=True)
    return {"path": path, "replacements": 1, "sha256": hashlib.sha256(result).hexdigest()}


class PathDocumentSource:
    """Interim MCP file adapter for one exact, already-visible Writer file.

    It only reads bytes for a private scratch copy and commits exported bytes;
    the headless engine never sees this path. Under the OS task-UID model an
    owner-private file may be unreadable and replacing a bind-mounted original
    is unsupported, so this is not a managed-save guarantee. A future OS staged
    read/commit capability replaces this class without changing any tool.
    """

    def __init__(self, path, *, edit):
        if not isinstance(path, str) or not path or any(ch in path for ch in "*?[\0"):
            raise ToolError("invalid_target", "Name one exact DOCX or ODT file; wildcards are not accepted.")
        self.edit = edit
        self.path = safe_realpath(path)
        self.suffix = Path(self.path).suffix.lower()
        if self.suffix not in DOCUMENT_MEMBERS:
            raise ToolError("invalid_target", "Yu Office Writer tools support DOCX and ODT documents only.")
        policy.require("fs.read", path=self.path)
        if edit:
            policy.require("fs.write", path=self.path)
        try:
            fd = os.open(self.path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        except FileNotFoundError:
            raise ToolError("missing_target", "The document does not exist.") from None
        except PermissionError:
            raise ToolError("unreadable_target", "Yu Office cannot read this document.") from None
        try:
            info = os.fstat(fd)
        except BaseException:
            os.close(fd)
            raise
        if not stat.S_ISREG(info.st_mode):
            os.close(fd)
            raise ToolError("invalid_target", "The document must be a regular file.")
        with os.fdopen(fd, "rb") as stream:
            if info.st_size > MAX_PACKAGE:
                raise ToolError("document_too_large", "The document must be no larger than 32 MiB.")
            data = stream.read(MAX_PACKAGE + 1)
            if len(data) > MAX_PACKAGE or _revision(os.fstat(stream.fileno())) != _revision(info):
                raise ToolError("stale_revision", "The document changed while reading it; retry.")
        if edit:
            try:
                with ZipFile(io.BytesIO(data)) as archive:
                    names = [name.lower() for name in archive.namelist()]
            except BadZipFile:
                raise ToolError("unreadable_document", "The document is not a DOCX or ODT package.") from None
            if any(name.startswith("_xmlsignatures/") or (name.startswith("meta-inf/") and "signatures" in name)
                   for name in names):
                raise ToolError("unsupported_content", "Yu Office does not change signed documents.")
        self.info, self.data = info, data

    def read(self):
        return self.data

    def commit(self, data):
        if not self.edit:
            raise ToolError("unauthorized_target", "This call may not change the document.")
        if len(data) > MAX_PACKAGE:
            raise ToolError("document_too_large", "The updated document exceeds 32 MiB.")
        try:
            if _revision(os.stat(self.path, follow_symlinks=False)) != _revision(self.info):
                raise ToolError("stale_revision", "The document changed on disk; inspect it again.")
            snapshot.snapshot(self.path, "write", session_id=current_context().session_id)
            if _revision(os.stat(self.path, follow_symlinks=False)) != _revision(self.info):
                raise ToolError("stale_revision", "The document changed on disk; inspect it again.")
            atomic_write_bytes(self.path, data, mode=stat.S_IMODE(self.info.st_mode) & 0o777, strict=True)
        except OSError as error:
            raise ToolError("save_unavailable",
                            f"Yu Office could not save this document in place ({error.strerror or error}).") from None
        return {"saved": True, "path": self.path, "sha256": hashlib.sha256(data).hexdigest()}


def writer_tool(catalog, name):
    """Bind one catalog tool to the shared handler with an MCP file target."""

    def handler(**arguments):
        catalog.validate(name, arguments, bound_target=False)
        source = PathDocumentSource(arguments["path"], edit=name == writer_tools.APPLY)
        with HeadlessOffice(PROGRAM_DIR) as office:
            result = writer_tools.call(catalog, name, arguments, FileTarget(source, office))
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
