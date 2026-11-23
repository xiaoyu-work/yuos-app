"""Review and owner-approve third-party App .deb submissions for the Yu OS Store.

Nothing submitted by a third party is ever auto-accepted or auto-published.
External developers build and pack a `.deb` locally (`yu app pack <dir>`) and
submit it to `submissions/incoming/`. The Store owner inspects it with `review`
(without executing any code inside the package) and signs + repacks it with
`approve` only after explicit approval.
"""

from __future__ import annotations

import argparse
import ast
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import stat
import sys
import tempfile

TOOLS_DIR = Path(__file__).resolve().parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

from release_common import command_environment, control_bytes, digest, fields, json_bytes, run, version, write_json
from release_payload import archive_path, deb_archive, validate_deb, validate_install_tree
import release_snapshots as snapshots
from web_ui import DESCRIPTOR

APP_ID_RE = re.compile(r"[a-z][a-z0-9_-]*")
DESKTOP_FILE_RE = re.compile(r"[a-zA-Z0-9][a-zA-Z0-9._-]{0,254}\.desktop")
SUPPORTED_ARCHITECTURES = ("all", "amd64", "arm64")
REQUIRED_RUNTIME_DEPENDENCY = "yuos-app-runtime-v1 (>= 0.1.0)"
DEFAULT_SOURCE_DATE_EPOCH = 1700000000

PROVENANCE_SCHEMA = "yuos.provenance/v1"
SIGNING_KEY_SCHEMA = "yuos.signing-key/v1"
MANIFEST_SCHEMA_V1 = "yuos.app-manifest/v1"
ALG_ED25519 = "ed25519"
ENVELOPE_FILE = ".provenance.json"

KEY_ID_DOMAIN = b"yuos-provenance/v1\x00publisher-key\x00"
TREE_DOMAIN = b"yuos-provenance/v1\x00content-tree\x00"
ENVELOPE_DOMAIN = b"yuos-provenance/v1\x00package-envelope\x00"

FORBIDDEN_AI_IMPORTS = (
    "openai",
    "anthropic",
    "google.generativeai",
    "vertexai",
    "cohere",
    "mistralai",
    "replicate",
    'boto3.client("bedrock',
    "boto3.client('bedrock",
)

# Exact App permission set from Yu-OS core/src/caps/catalog.rs::APP_PERMISSIONS.
APP_PERMISSION_VERBS = frozenset({
    "fs.read", "fs.write", "fs.delete", "fs.watch", "fs.meta",
    "net.dial", "net.listen", "net.resolve", "net.probe",
    "data.kv.read", "data.kv.write", "data.kv.delete",
    "data.log.write",
    "memory.write", "memory.read",
    "ui.notify", "ui.prompt", "ui.window",
    "clipboard.read", "clipboard.write",
    "device.audio", "device.bluetooth", "device.printer",
    "device.camera", "device.microphone", "device.location", "device.sensor",
    "time.cron", "time.delay",
    "ai.chat", "ai.chat.untrusted", "ai.embed", "ai.decide",
    "ai.image.generate", "ai.image.analyze",
    "ai.audio.tts", "ai.audio.stt",
    "ai.vision.analyze",
    "ai.video.generate", "ai.video.analyze",
    "desktop.capture", "desktop.media.observe", "desktop.media.control",
})

ALL_KNOWN_VERBS = frozenset({
    *APP_PERMISSION_VERBS,
    "fs.exec",
    "net.raw", "net.manage", "net.firewall",
    "proc.spawn", "proc.signal", "proc.observe",
    "sys.observe", "sys.crash", "sys.container", "sys.config", "sys.events",
    "sys.permissions", "sys.identity", "sys.locale", "sys.language", "sys.hostname",
    "sys.security", "sys.storage", "sys.service", "sys.package", "sys.mount",
    "sys.snapshot", "sys.time", "sys.power", "sys.kernel",
    "secret.read", "secret.write", "secret.grant",
    "agent.spawn", "agent.observe", "agent.delegate",
    "data.db.read", "data.db.write", "data.log.read",
    "data.inbox.read", "data.inbox.write", "data.backup",
    "ipc.publish", "ipc.subscribe", "ipc.invoke",
    "ui.panel", "ui.input", "ui.accessibility",
    "device.media.route", "device.display", "device.usb",
    "ai.bypass",
    "desktop.launch", "desktop.window",
    "browser.tabs.read", "browser.nav", "browser.dom.read", "browser.dom.write",
    "browser.input.secret", "browser.eval",
})


@contextmanager
def _posix_scratch(prefix: str = ".store-submission-"):
    """Use a scratch directory that preserves POSIX permission bits (e.g. /tmp on WSL)."""
    candidates = []
    if Path("/tmp").is_dir():
        candidates.append(Path("/tmp"))
    candidates.append(Path(tempfile.gettempdir()))
    selected = None
    for base in candidates:
        try:
            with tempfile.TemporaryDirectory(prefix=prefix, dir=base) as probe:
                probe_path = Path(probe)
                probe_path.chmod(0o755)
                if stat.S_IMODE(probe_path.stat().st_mode) == 0o755:
                    selected = base
                    break
        except OSError:
            continue
    with tempfile.TemporaryDirectory(prefix=prefix, dir=selected) as scratch:
        path = Path(scratch)
        path.chmod(0o755)
        yield path


class _Canon:
    """Length-prefixed, domain-separated canonical encoder matching Yu-OS provenance."""

    def __init__(self, domain: bytes):
        self.out = bytearray()
        self.out += len(domain).to_bytes(8, "little")
        self.out += domain

    def bytes_field(self, key: str, value: bytes) -> _Canon:
        key_bytes = key.encode("utf-8")
        self.out += len(key_bytes).to_bytes(4, "little")
        self.out += key_bytes
        self.out += len(value).to_bytes(8, "little")
        self.out += value
        return self

    def text(self, key: str, value: str) -> _Canon:
        return self.bytes_field(key, value.encode("utf-8"))

    def uint(self, key: str, value: int) -> _Canon:
        return self.bytes_field(key, int(value).to_bytes(8, "little"))

    def finish(self) -> bytes:
        return bytes(self.out)


def key_id_for(public_key: bytes) -> str:
    if len(public_key) != 32:
        raise ValueError("Ed25519 verifying key must be 32 bytes")
    return "sha256:" + hashlib.sha256(KEY_ID_DOMAIN + public_key).hexdigest()


def content_digest_bytes(files: list[dict]) -> bytes:
    canon = _Canon(TREE_DOMAIN)
    canon.uint("count", len(files))
    for item in files:
        canon.text("path", item["path"])
        canon.text("type", item["type"])
        canon.uint("mode", item["mode"] & 0o7777)
        canon.uint("size", item["size"])
        canon.text("digest", item["digest"])
    return canon.finish()


def content_digest(files: list[dict]) -> str:
    return "sha256:" + hashlib.sha256(content_digest_bytes(files)).hexdigest()


def canonical_envelope_bytes(body: dict, algorithm: str, key_id: str, public_key: str) -> bytes:
    canon = _Canon(ENVELOPE_DOMAIN)
    canon.text("schema", PROVENANCE_SCHEMA)
    canon.text("algorithm", algorithm)
    canon.text("key_id", key_id)
    canon.text("public_key", public_key)
    canon.text("kind", body["kind"])
    canon.text("id", body["id"])
    canon.text("version", body["version"])
    canon.text("manifest_schema", body["manifest_schema"])
    canon.text("manifest_path", body["manifest_path"])
    canon.uint("entrypoints", len(body["entrypoints"]))
    for entry in body["entrypoints"]:
        canon.text("entrypoint", entry)
    canon.uint("resources", len(body["resources"]))
    for resource in body["resources"]:
        canon.text("resource", resource)
    canon.bytes_field("tree", content_digest_bytes(body["files"]))
    canon.text("content_digest", body["content_digest"])
    return canon.finish()


def generate_signing_key(path: Path | None = None, *, comment: str | None = None) -> dict:
    """Generate a yuos.signing-key/v1 Ed25519 signing key."""
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    from cryptography.hazmat.primitives.serialization import Encoding, NoEncryption, PrivateFormat, PublicFormat

    private = Ed25519PrivateKey.generate()
    seed = private.private_bytes(Encoding.Raw, PrivateFormat.Raw, NoEncryption())
    public = private.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw)
    key_data = {
        "schema": SIGNING_KEY_SCHEMA,
        "algorithm": ALG_ED25519,
        "key_id": key_id_for(public),
        "public_key": public.hex(),
        "private_key": seed.hex(),
    }
    if comment is not None:
        key_data["comment"] = comment
    if path is not None:
        path = Path(path)
        path.parent.mkdir(parents=True, exist_ok=True)
        write_json(path, key_data)
        path.chmod(0o600)
    return key_data


def load_signing_key(path: Path) -> tuple[dict, bytes, bytes, str]:
    path = Path(path)
    data = snapshots.read_metadata(path)
    if data.get("schema") != SIGNING_KEY_SCHEMA or data.get("algorithm") != ALG_ED25519:
        raise ValueError("Provenance signing key requires schema yuos.signing-key/v1 and algorithm ed25519")
    raw_private = data.get("private_key", "")
    if not isinstance(raw_private, str) or not re.fullmatch(r"[0-9a-f]{64}", raw_private):
        raise ValueError("Provenance signing key requires a 32-byte lowercase hex private_key")
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    from cryptography.hazmat.primitives.serialization import Encoding, PublicFormat

    seed = bytes.fromhex(raw_private)
    private = Ed25519PrivateKey.from_private_bytes(seed)
    public = private.public_key().public_bytes(Encoding.Raw, PublicFormat.Raw)
    computed_key_id = key_id_for(public)
    if "public_key" in data and data["public_key"] != public.hex():
        raise ValueError("Provenance signing key public_key does not match private_key")
    if "key_id" in data and data["key_id"] != computed_key_id:
        raise ValueError("Provenance signing key key_id does not match public_key")
    return data, seed, public, computed_key_id


def _collect_provenance_files(app_root: Path) -> list[dict]:
    files = []
    for path in sorted(app_root.rglob("*"), key=lambda item: item.relative_to(app_root).as_posix()):
        rel = path.relative_to(app_root).as_posix()
        if rel == ENVELOPE_FILE:
            continue
        snapshots.app_path(rel)
        info = path.lstat()
        mode = stat.S_IMODE(info.st_mode) & 0o7777
        if stat.S_ISLNK(info.st_mode):
            raise ValueError(f"yuos.provenance/v1 cannot sign symlinks: {rel}")
        if mode & 0o7022:
            raise ValueError(f"Unsafe App payload mode: {rel}")
        if stat.S_ISDIR(info.st_mode):
            files.append({
                "path": rel,
                "type": "dir",
                "mode": mode,
                "size": 0,
                "digest": "",
            })
        elif stat.S_ISREG(info.st_mode):
            files.append({
                "path": rel,
                "type": "file",
                "mode": mode,
                "size": info.st_size,
                "digest": "sha256:" + digest(path),
            })
        else:
            raise ValueError(f"Special files cannot be signed: {rel}")
    return files


def _declared_resources(app_root: Path) -> list[str]:
    resources = []
    if (app_root / DESCRIPTOR).is_file() and (app_root / "web/index.html").is_file():
        resources.extend((DESCRIPTOR, "web/index.html"))
    desktop_dir = app_root / "resources/usr/share/applications"
    if desktop_dir.is_dir():
        for desktop in sorted(desktop_dir.glob("*.desktop")):
            if desktop.is_symlink() or not desktop.is_file() or not DESKTOP_FILE_RE.fullmatch(desktop.name):
                raise ValueError(f"Invalid desktop resource in App payload: {desktop.name}")
            resources.append(desktop.relative_to(app_root).as_posix())
    return resources


def sign_app_directory(
    app_root: Path,
    manifest: dict,
    key_path: Path,
    *,
    yu_bin: Path | None = None,
) -> dict:
    """Sign an extracted App directory with `yu provenance sign` or `yuos.provenance/v1`."""
    app_root = Path(app_root)
    envelope_path = app_root / ENVELOPE_FILE
    if envelope_path.exists() or envelope_path.is_symlink():
        envelope_path.unlink()
    entrypoints = [entry for _, entry in snapshots.manifest_entries(manifest)]
    resources = _declared_resources(app_root)
    if yu_bin is not None:
        yu_bin = Path(yu_bin).resolve(strict=True)
        arguments = [
            yu_bin, "provenance", "sign", "--kind", "app",
            "--id", manifest["id"], "--version", manifest["version"],
            "--path", app_root, "--key", Path(key_path).resolve(strict=True),
        ]
        for entry in entrypoints:
            arguments.extend(("--entrypoint", entry))
        for resource in resources:
            arguments.extend(("--resource", resource))
        run(arguments)
        return verify_provenance_envelope(app_root)

    _, seed, public, key_id = load_signing_key(key_path)
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

    files = _collect_provenance_files(app_root)
    body = {
        "kind": "app",
        "id": manifest["id"],
        "version": manifest["version"],
        "manifest_schema": MANIFEST_SCHEMA_V1,
        "manifest_path": "app.json",
        "entrypoints": entrypoints,
        "resources": resources,
        "files": files,
        "content_digest": content_digest(files),
    }
    public_hex = public.hex()
    message = canonical_envelope_bytes(body, ALG_ED25519, key_id, public_hex)
    signature = Ed25519PrivateKey.from_private_bytes(seed).sign(message)
    envelope = {
        "schema": PROVENANCE_SCHEMA,
        "package": body,
        "signature": {
            "algorithm": ALG_ED25519,
            "key_id": key_id,
            "public_key": public_hex,
            "value": signature.hex(),
        },
    }
    write_json(envelope_path, envelope)
    envelope_path.chmod(0o644)
    return verify_provenance_envelope(app_root, expected_key_id=key_id)


def verify_provenance_envelope(app_root: Path, *, expected_key_id: str | None = None) -> dict:
    """Verify a signed `.provenance.json` envelope and file inventory in `app_root`."""
    app_root = Path(app_root)
    envelope_path = app_root / ENVELOPE_FILE
    envelope = snapshots.read_metadata(envelope_path)
    manifest = snapshots.read_metadata(app_root / "app.json")
    package = envelope.get("package", {})
    signature = envelope.get("signature", {})
    if (
        envelope.get("schema") != PROVENANCE_SCHEMA
        or not isinstance(package, dict)
        or not isinstance(signature, dict)
        or package.get("kind") != "app"
        or package.get("manifest_schema") not in (MANIFEST_SCHEMA_V1, "yuos.app/v2")
        or package.get("manifest_path") != "app.json"
        or signature.get("algorithm") != ALG_ED25519
    ):
        raise ValueError("App provenance envelope does not match yuos.provenance/v1")
    for field in ("id", "version"):
        if manifest.get(field) != package.get(field):
            raise ValueError(f"App provenance {field} does not match app.json")
    entrypoints = package.get("entrypoints")
    if not isinstance(entrypoints, list) or not entrypoints:
        raise ValueError("App provenance requires signed entrypoints")
    snapshots.validate_manifest_entries(app_root, manifest, entrypoints)
    actual_files = _collect_provenance_files(app_root)
    if package.get("files") != actual_files:
        raise ValueError("App payload files differ from signed provenance inventory")
    expected_content_digest = content_digest(actual_files)
    if package.get("content_digest") != expected_content_digest:
        raise ValueError("App provenance content_digest mismatch")
    key_id = snapshots.sha256_value(signature.get("key_id", ""), prefixed=True)
    if expected_key_id is not None and key_id != expected_key_id:
        raise ValueError("App provenance publisher key_id mismatch")
    if "public_key" in signature:
        raw_public = signature["public_key"]
        if not isinstance(raw_public, str) or not re.fullmatch(r"[0-9a-f]{64}", raw_public):
            raise ValueError("Invalid Ed25519 public_key in provenance signature")
        public_bytes = bytes.fromhex(raw_public)
        if key_id_for(public_bytes) != key_id:
            raise ValueError("Provenance key_id does not match signature public_key")
        raw_sig = signature.get("value", "")
        if not isinstance(raw_sig, str) or not re.fullmatch(r"[0-9a-f]{128}", raw_sig):
            raise ValueError("Invalid Ed25519 signature value in provenance envelope")
        from cryptography.exceptions import InvalidSignature
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

        message = canonical_envelope_bytes(package, ALG_ED25519, key_id, raw_public)
        try:
            Ed25519PublicKey.from_public_bytes(public_bytes).verify(bytes.fromhex(raw_sig), message)
        except InvalidSignature as error:
            raise ValueError("Ed25519 signature verification failed for .provenance.json") from error
    return envelope


def _validate_control(deb_path: Path) -> tuple[dict, str, str, str]:
    raw_control = run(["dpkg-deb", "--field", deb_path]).stdout.decode("utf-8", errors="replace").strip()
    control = fields(raw_control)
    for required in ("Package", "Version", "Architecture", "Depends", "X-YuOS-App-Ids"):
        if required not in control or not control[required].strip():
            raise ValueError(f"Missing required DEBIAN/control field: {required}")
    package = control["Package"].strip()
    if not package.startswith("yuos-app-"):
        raise ValueError(f"Package must be named yuos-app-<id>, got: {package!r}")
    app_id = package.removeprefix("yuos-app-")
    if not APP_ID_RE.fullmatch(app_id):
        raise ValueError(f"Invalid App ID in Package field: {app_id!r}")
    pkg_version = version(control["Version"].strip())
    architecture = control["Architecture"].strip()
    if architecture not in SUPPORTED_ARCHITECTURES:
        raise ValueError(f"Unsupported package Architecture: {architecture!r}")
    depends = [item.strip() for item in control["Depends"].replace("\n", " ").split(",") if item.strip()]
    if REQUIRED_RUNTIME_DEPENDENCY not in depends:
        raise ValueError(
            f"DEBIAN/control Depends must include {REQUIRED_RUNTIME_DEPENDENCY!r}"
        )
    declared_ids = [item.strip() for item in control["X-YuOS-App-Ids"].split(",") if item.strip()]
    if declared_ids != [app_id]:
        raise ValueError(
            f"DEBIAN/control X-YuOS-App-Ids must be {app_id!r}, got {control['X-YuOS-App-Ids']!r}"
        )
    return control, app_id, pkg_version, architecture


def _validate_submission_paths(deb_path: Path, app_id: str, scratch: Path) -> None:
    app_prefix = PurePosixPath(f"usr/lib/yuos/apps/{app_id}")
    desktop_prefix = PurePosixPath("usr/share/applications")
    doc_prefix = PurePosixPath(f"usr/share/doc/yuos-app-{app_id}")
    pkg_meta_file = PurePosixPath(f"usr/share/yuos-app/packages/yuos-app-{app_id}.json")
    allowed_ancestors = {
        PurePosixPath("."),
        PurePosixPath("usr"),
        PurePosixPath("usr/lib"),
        PurePosixPath("usr/lib/yuos"),
        PurePosixPath("usr/lib/yuos/apps"),
        PurePosixPath("usr/share"),
        PurePosixPath("usr/share/applications"),
        PurePosixPath("usr/share/doc"),
        PurePosixPath("usr/share/yuos-app"),
        PurePosixPath("usr/share/yuos-app/packages"),
    }
    saw_manifest = False
    with deb_archive(deb_path, "--fsys-tarfile", command_environment(), scratch) as archive:
        for member in archive:
            path = archive_path(member.name)
            if path in allowed_ancestors:
                if not member.isdir():
                    raise ValueError(f"Expected directory at ancestor path: {path}")
                continue
            if path == app_prefix or path.is_relative_to(app_prefix):
                if path == app_prefix / "app.json" and member.isfile():
                    saw_manifest = True
                continue
            if path.is_relative_to(desktop_prefix):
                rel = path.relative_to(desktop_prefix)
                if len(rel.parts) != 1 or not member.isfile() or not DESKTOP_FILE_RE.fullmatch(rel.name):
                    raise ValueError(f"Invalid desktop entry outside usr/lib/yuos/apps/{app_id}: {path}")
                continue
            if path == doc_prefix or path.is_relative_to(doc_prefix):
                continue
            if path == pkg_meta_file and member.isfile():
                continue
            raise ValueError(
                f"Submission payload contains path outside usr/lib/yuos/apps/{app_id}: {path}"
            )
    if not saw_manifest:
        raise ValueError(f"Submission is missing usr/lib/yuos/apps/{app_id}/app.json")


def _validate_md5sums(root: Path) -> None:
    sums_path = root / "DEBIAN/md5sums"
    if not sums_path.exists():
        return
    recorded = {}
    for line in sums_path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        parts = line.split("  ", 1)
        if len(parts) != 2 or not re.fullmatch(r"[0-9a-f]{32}", parts[0]):
            raise ValueError(f"Malformed DEBIAN/md5sums line: {line!r}")
        rel = archive_path(parts[1]).as_posix()
        if rel in recorded:
            raise ValueError(f"Duplicate DEBIAN/md5sums entry: {rel}")
        recorded[rel] = parts[0]
    actual = {
        path.relative_to(root).as_posix(): digest(path, "md5")
        for path in sorted(root.rglob("*"))
        if path.is_file() and not path.is_symlink() and not path.is_relative_to(root / "DEBIAN")
    }
    if recorded != actual:
        raise ValueError("DEBIAN/md5sums does not match extracted payload files")


def _validate_manifest_schema(manifest: dict, app_id: str) -> None:
    if manifest.get("id") != app_id:
        raise ValueError(f"Manifest id {manifest.get('id')!r} does not match package App ID {app_id!r}")
    manifest_version = manifest.get("version")
    if not isinstance(manifest_version, str) or not manifest_version.strip():
        raise ValueError("Manifest requires a non-empty string version")
    name = manifest.get("name")
    if not (
        (isinstance(name, str) and name.strip())
        or (
            isinstance(name, dict)
            and isinstance(name.get("en"), str)
            and name["en"].strip()
            and all(isinstance(v, str) for v in name.values())
        )
    ):
        raise ValueError("Manifest requires a non-empty name or localized name with 'en'")
    runtime = manifest.get("runtime", "python")
    if runtime not in snapshots.RUNTIME_ENTRIES:
        raise ValueError(f"Unsupported manifest runtime: {runtime!r}")

    # Validate against the public JSON schema when the platform cache is available.
    try:
        import platform_dependency
        from jsonschema import Draft202012Validator

        schema_path = platform_dependency.manifest_schema_path(download=False)
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
        Draft202012Validator(schema).validate(manifest)
    except FileNotFoundError:
        pass


def _is_own_app_event(need: dict, app_id: str) -> bool:
    if need.get("verb") != "ipc.publish":
        return False
    scope = need.get("scope")
    if not isinstance(scope, dict):
        return False
    if scope.get("kind") == "fixed":
        inner = scope.get("scope", {})
        return isinstance(inner, dict) and inner.get("kind") == "name" and inner.get("value") == f"app:{app_id}"
    return scope.get("kind") == "name" and scope.get("value") == f"app:{app_id}"


def _permission_category(verb: str, is_app_perm: bool) -> str:
    if not is_app_perm:
        return "system"
    if verb.startswith("ai."):
        return "ai"
    if verb.startswith(("device.", "desktop.media.", "desktop.capture")):
        return "device"
    if verb.startswith(("fs.", "data.", "memory.", "clipboard.")):
        return "privacy"
    if verb.startswith("net."):
        return "network"
    return "ui"


def _why_text(why) -> str:
    if isinstance(why, str):
        return why
    if isinstance(why, dict):
        return str(why.get("en") or next(iter(why.values()), ""))
    return ""


def _collect_permissions(manifest: dict, app_id: str) -> dict:
    surfaces: list[tuple[str, str, bool, list]] = []
    if "needs" in manifest:
        if not isinstance(manifest["needs"], list):
            raise ValueError("Manifest top-level needs must be a list")
        surfaces.append(("operations", "manifest", False, manifest["needs"]))
    if "optional_needs" in manifest:
        if not isinstance(manifest["optional_needs"], list):
            raise ValueError("Manifest top-level optional_needs must be a list")
        surfaces.append(("operations", "manifest", True, manifest["optional_needs"]))

    operations = manifest.get("operations", {})
    if not isinstance(operations, dict):
        raise ValueError("Manifest operations must be an object")
    for op_name, op_decl in sorted(operations.items()):
        if not isinstance(op_decl, dict):
            raise ValueError(f"Operation {op_name!r} must be an object")
        for field_name, optional in (("needs", False), ("optional_needs", True)):
            if field_name in op_decl:
                if not isinstance(op_decl[field_name], list):
                    raise ValueError(f"Operation {op_name!r} {field_name} must be a list")
                surfaces.append(("operations", f"operation:{op_name}", optional, op_decl[field_name]))

    if "desktop" in manifest:
        desktop = manifest["desktop"]
        if not isinstance(desktop, dict):
            raise ValueError("Manifest desktop must be an object")
        for field_name, optional in (("needs", False), ("optional_needs", True)):
            if field_name in desktop:
                if not isinstance(desktop[field_name], list):
                    raise ValueError(f"Desktop {field_name} must be a list")
                surfaces.append(("desktop", "desktop", optional, desktop[field_name]))
        entries = desktop.get("entries", {})
        if isinstance(entries, dict):
            for entry_name, entry_decl in sorted(entries.items()):
                if not isinstance(entry_decl, dict):
                    continue
                for field_name, optional in (("needs", False), ("optional_needs", True)):
                    if field_name in entry_decl:
                        if not isinstance(entry_decl[field_name], list):
                            raise ValueError(f"Desktop entry {entry_name!r} {field_name} must be a list")
                        surfaces.append(("desktop", f"desktop:{entry_name}", optional, entry_decl[field_name]))

    if "mcp" in manifest:
        mcp = manifest["mcp"]
        if not isinstance(mcp, dict):
            raise ValueError("Manifest mcp must be an object")
        tools = mcp.get("tools", [])
        if not isinstance(tools, list):
            raise ValueError("Manifest mcp.tools must be a list")
        for tool in tools:
            if not isinstance(tool, dict) or not isinstance(tool.get("name"), str):
                raise ValueError("Each MCP tool must be an object with a string name")
            tool_name = tool["name"]
            for field_name, optional in (("needs", False), ("optional_needs", True)):
                if field_name in tool:
                    if not isinstance(tool[field_name], list):
                        raise ValueError(f"MCP tool {tool_name!r} {field_name} must be a list")
                    surfaces.append(("mcp.tools", f"mcp:{tool_name}", optional, tool[field_name]))

    app_permissions = []
    system_permissions = []
    by_surface: dict[str, list[dict]] = {"operations": [], "desktop": [], "mcp.tools": []}

    for group, surface_label, optional, needs in surfaces:
        for need in needs:
            if not isinstance(need, dict):
                raise ValueError(f"Permission declaration in {surface_label} must be an object")
            verb = need.get("verb")
            if not isinstance(verb, str) or verb not in ALL_KNOWN_VERBS:
                raise ValueError(f"Unknown or invalid capability verb in {surface_label}: {verb!r}")
            scope = need.get("scope")
            if scope is not None and not isinstance(scope, dict):
                raise ValueError(f"Capability scope in {surface_label} must be an object")
            if scope is None and not verb.startswith("ai."):
                raise ValueError(f"Capability {verb!r} in {surface_label} requires an explicit scope")
            is_app = verb in APP_PERMISSION_VERBS or _is_own_app_event(need, app_id)
            record = {
                "verb": verb,
                "tier": "app" if is_app else "system",
                "category": _permission_category(verb, is_app),
                "surface": surface_label,
                "optional": optional,
                "scope": scope or {"kind": "user-configured-model"},
                "why": _why_text(need.get("why")),
                "flagged": not is_app,
            }
            by_surface[group].append(record)
            if is_app:
                app_permissions.append(record)
            else:
                system_permissions.append(record)

    return {
        "app_permissions": app_permissions,
        "system_permissions": system_permissions,
        "by_surface": by_surface,
        "has_system_permissions": bool(system_permissions),
        "flagged_system_verbs": sorted({item["verb"] for item in system_permissions}),
    }


def _lint_ai_imports(app_root: Path, manifest: dict, permissions: dict) -> None:
    declares_ai = "ai" in manifest or any(
        item["verb"].startswith("ai.")
        for item in (*permissions["app_permissions"], *permissions["system_permissions"])
    )
    if not declares_ai:
        return
    hits = []
    for py_file in sorted(app_root.rglob("*.py")):
        rel = py_file.relative_to(app_root).as_posix()
        source = py_file.read_text(encoding="utf-8", errors="replace")
        try:
            tree = ast.parse(source, filename=rel)
            for node in ast.walk(tree):
                if isinstance(node, ast.Import):
                    for alias in node.names:
                        for forbidden in FORBIDDEN_AI_IMPORTS:
                            if not forbidden.startswith("boto3.") and (
                                alias.name == forbidden or alias.name.startswith(forbidden + ".")
                            ):
                                hits.append(f"{rel}:{node.lineno} imports {forbidden}")
                elif isinstance(node, ast.ImportFrom) and node.module:
                    for forbidden in FORBIDDEN_AI_IMPORTS:
                        if not forbidden.startswith("boto3.") and (
                            node.module == forbidden or node.module.startswith(forbidden + ".")
                        ):
                            hits.append(f"{rel}:{node.lineno} imports from {forbidden}")
        except SyntaxError as error:
            raise ValueError(f"Invalid Python syntax in {rel}:{error.lineno}: {error.msg}") from error

        for index, line in enumerate(source.splitlines(), start=1):
            trimmed = line.lstrip()
            for needle in FORBIDDEN_AI_IMPORTS:
                if needle in trimmed and (
                    trimmed.startswith(("import ", "from ")) or ".client" in trimmed
                ):
                    entry = f"{rel}:{index} uses forbidden provider SDK {needle}"
                    if entry not in hits:
                        hits.append(entry)
    if hits:
        raise ValueError(
            "Apps declaring AI must use the Yu OS SDK instead of direct model-provider SDKs: "
            + "; ".join(hits)
        )


def review_deb(deb_path: Path) -> dict:
    """Validate an untrusted submitted `.deb` without executing code and return a review report."""
    deb_path = Path(deb_path)
    if deb_path.is_symlink() or not deb_path.is_file():
        raise ValueError(f"Submission must be a regular .deb file: {deb_path}")

    with _posix_scratch() as scratch:
        validate_deb(deb_path, environment=command_environment(), scratch=scratch)
        control, app_id, pkg_version, architecture = _validate_control(deb_path)
        _validate_submission_paths(deb_path, app_id, scratch)

        extracted = scratch / "extracted"
        run(["dpkg-deb", "--raw-extract", deb_path, extracted])
        validate_install_tree(extracted)
        _validate_md5sums(extracted)

        app_root = extracted / "usr/lib/yuos/apps" / app_id
        if app_root.is_symlink() or not app_root.is_dir():
            raise ValueError(f"Missing extracted App directory: usr/lib/yuos/apps/{app_id}")

        manifest = snapshots.read_metadata(app_root / "app.json")
        _validate_manifest_schema(manifest, app_id)
        if pkg_version.split("-")[0] != manifest["version"] and pkg_version != manifest["version"]:
            raise ValueError(
                f"DEBIAN/control Version {pkg_version!r} does not match manifest version {manifest['version']!r}"
            )

        entries = snapshots.manifest_entries(manifest)
        snapshots.validate_manifest_entries(
            app_root, manifest, [entry for _, entry in entries]
        )
        inventory = snapshots.tree_inventory(app_root, architecture)

        provenance_present = (app_root / ENVELOPE_FILE).exists()
        regular_files = [
            (name, record)
            for name, record in inventory.items()
            if record["type"] == "file" and name != ENVELOPE_FILE
        ]
        file_count = len(regular_files)
        total_size = sum(record["size"] for _, record in regular_files)

        permissions = _collect_permissions(manifest, app_id)
        _lint_ai_imports(app_root, manifest, permissions)

        provenance_note = (
            "Submitted package includes a developer .provenance.json; Store approval will strip "
            "and replace it with the Store publisher signature after owner review."
            if provenance_present
            else "No .provenance.json in submitted package (expected); Store approval will sign "
            "the App payload after owner review."
        )

        return {
            "ok": True,
            "app_id": app_id,
            "package": control["Package"].strip(),
            "version": manifest["version"],
            "package_version": pkg_version,
            "architecture": architecture,
            "runtime": manifest.get("runtime", "python"),
            "deb_path": str(deb_path),
            "deb_size": deb_path.stat().st_size,
            "deb_sha256": digest(deb_path),
            "entrypoints": [
                {"surface": surface, "path": entry}
                for surface, entry in entries
            ],
            "file_count": file_count,
            "total_size": total_size,
            "submitted_provenance_present": provenance_present,
            "provenance_note": provenance_note,
            "permissions": permissions,
        }


def format_review_report(report: dict) -> str:
    """Format a human-readable owner review summary."""
    lines = [
        f"=== Yu OS Store Submission Review: {report['app_id']} ===",
        f"Package:      {report['package']} ({report['package_version']})",
        f"App ID:       {report['app_id']}",
        f"App Version:  {report['version']}",
        f"Architecture: {report['architecture']}",
        f"Runtime:      {report['runtime']}",
        f"Archive:      {report['deb_path']} ({report['deb_size']} bytes, sha256:{report['deb_sha256']})",
        f"App Payload:  {report['file_count']} files, {report['total_size']} bytes total",
        f"Provenance:   {report['provenance_note']}",
        "",
        "Entrypoints:",
    ]
    for item in report["entrypoints"]:
        lines.append(f"  - [{item['surface']}] {item['path']}")

    perms = report["permissions"]
    lines.append("")
    lines.append(f"App Permissions (privacy/device/AI) [{len(perms['app_permissions'])}]:")
    if perms["app_permissions"]:
        for item in perms["app_permissions"]:
            opt = " (optional)" if item["optional"] else ""
            why = f" — {item['why']}" if item["why"] else ""
            lines.append(
                f"  - [{item['category']}] {item['verb']}{opt} on {item['surface']} "
                f"(scope: {json.dumps(item['scope'], sort_keys=True)}){why}"
            )
    else:
        lines.append("  (none)")

    lines.append("")
    if perms["has_system_permissions"]:
        lines.append(
            f"[WARNING: SYSTEM PERMISSIONS REQUESTED] System Permissions [{len(perms['system_permissions'])}]:"
        )
        for item in perms["system_permissions"]:
            opt = " (optional)" if item["optional"] else ""
            why = f" — {item['why']}" if item["why"] else ""
            lines.append(
                f"  ! [SYSTEM PERMISSION] {item['verb']}{opt} on {item['surface']} "
                f"(scope: {json.dumps(item['scope'], sort_keys=True)}){why}"
            )
    else:
        lines.append("System Permissions [0]:")
        lines.append("  (none)")

    return "\n".join(lines)


def _normalize_stage_tree(root: Path, epoch: int = DEFAULT_SOURCE_DATE_EPOCH) -> None:
    validate_install_tree(root)
    for path in [*root.rglob("*"), root]:
        mode = path.lstat().st_mode
        if stat.S_ISLNK(mode):
            if not path.resolve().is_relative_to(root.resolve()):
                raise ValueError(f"Package contains an escaping symlink: {path}")
        elif stat.S_ISDIR(mode):
            path.chmod(0o755)
        elif stat.S_ISREG(mode):
            if path.is_relative_to(root / "DEBIAN"):
                path.chmod(0o644)
            else:
                path.chmod(0o755 if mode & 0o111 else 0o644)
        else:
            raise ValueError(f"Package contains a special file: {path}")
        os.utime(path, (epoch, epoch), follow_symlinks=False)


def _write_md5sums(root: Path) -> None:
    sums_path = root / "DEBIAN/md5sums"
    content = "".join(
        f"{digest(path, 'md5')}  {path.relative_to(root).as_posix()}\n"
        for path in sorted(root.rglob("*"))
        if path.is_file() and not path.is_symlink() and not path.is_relative_to(root / "DEBIAN")
    )
    sums_path.write_text(content, encoding="utf-8")
    sums_path.chmod(0o644)


def approve_deb(
    deb_path: Path,
    out_path: Path,
    provenance_key: Path,
    *,
    yu_bin: Path | None = None,
) -> dict:
    """Review, strip untrusted provenance, sign with the publisher key, and repack a clean .deb."""
    deb_path = Path(deb_path)
    out_path = Path(out_path)
    provenance_key = Path(provenance_key)

    report = review_deb(deb_path)
    app_id = report["app_id"]
    architecture = report["architecture"]

    with _posix_scratch(prefix=".store-approve-") as scratch:
        stage_root = scratch / "stage"
        run(["dpkg-deb", "--raw-extract", deb_path, stage_root])
        validate_install_tree(stage_root)

        app_root = stage_root / "usr/lib/yuos/apps" / app_id
        untrusted_provenance = app_root / ENVELOPE_FILE
        if untrusted_provenance.exists() or untrusted_provenance.is_symlink():
            untrusted_provenance.unlink()

        _normalize_stage_tree(stage_root)
        manifest = snapshots.read_metadata(app_root / "app.json")
        envelope = sign_app_directory(app_root, manifest, provenance_key, yu_bin=yu_bin)

        _write_md5sums(stage_root)
        _normalize_stage_tree(stage_root)

        built_deb = scratch / f"{report['package']}_{report['package_version']}_{architecture}.deb"
        run(
            [
                "dpkg-deb", "--root-owner-group", "--uniform-compression",
                "--threads-max=1", "-Zxz", "-z9", "--build", stage_root, built_deb,
            ],
            env={"SOURCE_DATE_EPOCH": str(DEFAULT_SOURCE_DATE_EPOCH)},
        )

        validate_deb(built_deb, environment=command_environment(), scratch=scratch)
        verify_root = scratch / "verify"
        run(["dpkg-deb", "--raw-extract", built_deb, verify_root])
        validate_install_tree(verify_root)
        _validate_md5sums(verify_root)
        verified_app = verify_root / "usr/lib/yuos/apps" / app_id
        snapshots.tree_inventory(verified_app, architecture)
        verified_envelope = verify_provenance_envelope(
            verified_app, expected_key_id=envelope["signature"]["key_id"]
        )

        out_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(built_deb, out_path)
        validate_deb(out_path, environment=command_environment(), scratch=scratch)

        return {
            "approved": True,
            "app_id": app_id,
            "package": report["package"],
            "version": report["version"],
            "package_version": report["package_version"],
            "architecture": architecture,
            "input_deb": str(deb_path),
            "output_deb": str(out_path),
            "size": out_path.stat().st_size,
            "sha256": digest(out_path),
            "provenance": {
                "path": ENVELOPE_FILE,
                "sha256": digest(verified_app / ENVELOPE_FILE),
                "content_digest": verified_envelope["package"]["content_digest"],
                "publisher_key_id": verified_envelope["signature"]["key_id"],
            },
            "review": report,
        }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    review_parser = subparsers.add_parser(
        "review",
        help="Validate a submitted .deb without executing code and print the owner review report",
    )
    review_parser.add_argument("deb", type=Path, help="Path to the submitted .deb package")
    review_parser.add_argument("--json", action="store_true", help="Emit JSON review report")

    approve_parser = subparsers.add_parser(
        "approve",
        help="Run all review checks, sign the App payload with the publisher key, and repack a clean .deb",
    )
    approve_parser.add_argument("deb", type=Path, help="Path to the reviewed .deb package")
    approve_parser.add_argument("--out", required=True, type=Path, help="Output path for the signed .deb")
    approve_parser.add_argument(
        "--provenance-key", required=True, type=Path,
        help="Path to the Store publisher Ed25519 signing key JSON (yuos.signing-key/v1)",
    )
    approve_parser.add_argument(
        "--yu-bin", type=Path, default=None,
        help="Optional path to the `yu` CLI binary for provenance signing",
    )
    approve_parser.add_argument("--json", action="store_true", help="Emit JSON approval report")
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.command == "review":
        report = review_deb(args.deb)
        if args.json:
            print(json.dumps(report, indent=2, sort_keys=True))
        else:
            print(format_review_report(report))
        return 0
    if args.command == "approve":
        result = approve_deb(
            args.deb,
            args.out,
            args.provenance_key,
            yu_bin=args.yu_bin,
        )
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print(
                f"Approved {result['package']} ({result['package_version']}, {result['architecture']}) "
                f"-> {result['output_deb']} (sha256:{result['sha256']}, "
                f"publisher:{result['provenance']['publisher_key_id']})"
            )
        return 0
    parser.error(f"Unknown command: {args.command}")
    return 2


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
