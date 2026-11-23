"""Focused tests for the owner-reviewed Store .deb submission workflow."""

from __future__ import annotations

import json
from pathlib import Path
import shutil
import stat
import subprocess

import pytest

from release_common import command_environment, control_bytes, digest, json_bytes, run, write_json
from release_payload import validate_deb
import store_submission


def _pack_submission_deb(
    output_dir: Path,
    *,
    app_id: str = "hello-notes",
    version: str = "1.0.0",
    architecture: str = "all",
    manifest: dict | None = None,
    app_files: dict[str, bytes] | None = None,
    extra_files: dict[str, bytes] | None = None,
    control_overrides: dict[str, str] | None = None,
    control_hooks: dict[str, bytes] | None = None,
    file_modes: dict[str, int] | None = None,
) -> Path:
    """Build a .deb matching `yu app pack <dir>` inside a POSIX-capable staging tree."""
    if manifest is None:
        manifest = {
            "schema_version": 2,
            "id": app_id,
            "version": version,
            "name": {"en": "Hello Notes"},
            "runtime": "python",
            "operations": {
                "save": {
                    "label": {"en": "Save note"},
                    "args": [{"name": "path", "kind": "path", "binding": "positional", "required": True}],
                    "needs": [
                        {
                            "verb": "fs.write",
                            "scope": {"kind": "from-arg", "arg": "path"},
                            "why": {"en": "Write your selected note file."},
                        }
                    ],
                }
            },
            "desktop": {
                "entry": "main.py",
                "exec": "--gui",
                "needs": [
                    {
                        "verb": "ui.window",
                        "scope": {"kind": "wild"},
                        "why": {"en": "Open the notes window."},
                    }
                ],
                "optional_needs": [
                    {
                        "verb": "device.microphone",
                        "scope": {"kind": "wild"},
                        "why": {"en": "Record voice notes when requested."},
                    }
                ],
            },
            "mcp": {
                "entry": "server.py",
                "lifecycle": "lazy",
                "tools": [
                    {
                        "name": f"{app_id}.summarize",
                        "summary": {"en": "Summarize notes"},
                        "args": [{"name": "text", "kind": "text", "required": True}],
                        "needs": [
                            {
                                "verb": "ai.chat",
                                "scope": {"kind": "wild"},
                                "why": {"en": "Summarize note content with the user's configured model."},
                            }
                        ],
                    }
                ],
            },
        }

    default_code = (
        "raise RuntimeError('Submitted App code must never execute during Store review or approval')\n"
    ).encode("utf-8")
    payload = {
        "app.json": json_bytes(manifest),
        "main.py": default_code,
        "server.py": default_code,
        **(app_files or {}),
    }

    with store_submission._posix_scratch(prefix=".test-pack-") as scratch:
        stage = scratch / "stage"
        debian = stage / "DEBIAN"
        debian.mkdir(parents=True, mode=0o755)
        app_root = stage / "usr/lib/yuos/apps" / app_id
        app_root.mkdir(parents=True, mode=0o755)

        for rel, content in payload.items():
            target = app_root / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(content)

        for rel, content in (extra_files or {}).items():
            target = stage / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(content)

        for path in [stage, *stage.rglob("*")]:
            if path.is_dir():
                path.chmod(0o755)
            elif path.is_file():
                path.chmod(0o644)

        for rel, mode in (file_modes or {}).items():
            (stage / rel).chmod(mode)

        control = {
            "Package": f"yuos-app-{app_id}",
            "Version": version,
            "Architecture": architecture,
            "Section": "misc",
            "Priority": "optional",
            "Maintainer": "External Developer <dev@example.invalid>",
            "Depends": "yuos-app-runtime-v1 (>= 0.1.0)",
            "X-YuOS-App-Ids": app_id,
            "Description": f"Yu OS App submission fixture ({app_id})",
            **(control_overrides or {}),
        }
        (debian / "control").write_bytes(control_bytes(control))
        (debian / "control").chmod(0o644)

        sums = "".join(
            f"{digest(path, 'md5')}  {path.relative_to(stage).as_posix()}\n"
            for path in sorted(stage.rglob("*"))
            if path.is_file() and not path.is_relative_to(debian)
        )
        (debian / "md5sums").write_text(sums, encoding="utf-8")
        (debian / "md5sums").chmod(0o644)

        for hook_name, content in (control_hooks or {}).items():
            hook_path = debian / hook_name
            hook_path.write_bytes(content)
            hook_path.chmod(0o755)

        built = scratch / f"{control['Package']}_{version}_{architecture}.deb"
        run(
            ["dpkg-deb", "--root-owner-group", "--build", stage, built],
            env={"SOURCE_DATE_EPOCH": "1700000000"},
        )
        output_dir.mkdir(parents=True, exist_ok=True)
        destination = output_dir / built.name
        shutil.copyfile(built, destination)
        return destination


def test_review_valid_unsigned_deb_categorizes_app_and_flags_system_permissions(tmp_path, capsys):
    deb_path = _pack_submission_deb(tmp_path / "incoming")
    report = store_submission.review_deb(deb_path)

    assert report["ok"] is True
    assert report["app_id"] == "hello-notes"
    assert report["package"] == "yuos-app-hello-notes"
    assert report["version"] == "1.0.0"
    assert report["architecture"] == "all"
    assert report["file_count"] == 3
    assert report["total_size"] > 0
    assert report["submitted_provenance_present"] is False
    assert {item["path"] for item in report["entrypoints"]} == {"main.py", "server.py"}

    perms = report["permissions"]
    assert perms["has_system_permissions"] is False
    assert perms["system_permissions"] == []
    assert {item["verb"] for item in perms["app_permissions"]} == {
        "fs.write",
        "ui.window",
        "device.microphone",
        "ai.chat",
    }
    categories = {item["verb"]: item["category"] for item in perms["app_permissions"]}
    assert categories == {
        "fs.write": "privacy",
        "ui.window": "ui",
        "device.microphone": "device",
        "ai.chat": "ai",
    }

    # Add a system permission (sys.package) and a developer .provenance.json and verify flagging.
    sys_manifest = {
        "schema_version": 2,
        "id": "sys-tool",
        "version": "1.2.0",
        "name": {"en": "System Helper"},
        "runtime": "python",
        "mcp": {
            "entry": "server.py",
            "tools": [
                {
                    "name": "sys-tool.inspect",
                    "summary": {"en": "Inspect packages"},
                    "needs": [
                        {
                            "verb": "fs.read",
                            "scope": {"kind": "fixed", "scope": {"kind": "path", "value": "/usr/share"}},
                            "why": {"en": "Read shared metadata."},
                        },
                        {
                            "verb": "sys.package",
                            "scope": {"kind": "wild"},
                            "why": {"en": "Manage system packages."},
                        },
                    ],
                }
            ],
        },
    }
    flagged_deb = _pack_submission_deb(
        tmp_path / "incoming-sys",
        app_id="sys-tool",
        version="1.2.0",
        manifest=sys_manifest,
        app_files={".provenance.json": b'{"untrusted": true}\n'},
    )
    assert store_submission.main(["review", str(flagged_deb)]) == 0
    text_out = capsys.readouterr().out
    assert "[WARNING: SYSTEM PERMISSIONS REQUESTED]" in text_out
    assert "[SYSTEM PERMISSION] sys.package" in text_out
    assert "replace it with the Store publisher signature" in text_out

    assert store_submission.main(["review", str(flagged_deb), "--json"]) == 0
    flagged_report = json.loads(capsys.readouterr().out)
    assert flagged_report["submitted_provenance_present"] is True
    assert flagged_report["permissions"]["has_system_permissions"] is True
    assert flagged_report["permissions"]["flagged_system_verbs"] == ["sys.package"]
    assert [item["verb"] for item in flagged_report["permissions"]["app_permissions"]] == ["fs.read"]
    assert [item["verb"] for item in flagged_report["permissions"]["system_permissions"]] == ["sys.package"]


def test_review_rejects_postinst_hooks_escaping_paths_and_direct_ai_sdk_imports(tmp_path):
    hook_deb = _pack_submission_deb(
        tmp_path / "hook",
        control_hooks={"postinst": b"#!/bin/sh\nid > /tmp/pwned\n"},
    )
    with pytest.raises(ValueError, match="maintainer/root hooks"):
        store_submission.review_deb(hook_deb)

    outside_deb = _pack_submission_deb(
        tmp_path / "outside",
        extra_files={"usr/bin/outside-helper": b"#!/bin/sh\nexit 0\n"},
    )
    with pytest.raises(ValueError, match="outside usr/lib/yuos/apps/hello-notes"):
        store_submission.review_deb(outside_deb)

    other_app_deb = _pack_submission_deb(
        tmp_path / "other-app",
        extra_files={"usr/lib/yuos/apps/other-app/app.json": b"{}\n"},
    )
    with pytest.raises(ValueError, match="outside usr/lib/yuos/apps/hello-notes"):
        store_submission.review_deb(other_app_deb)

    direct_sdk_deb = _pack_submission_deb(
        tmp_path / "direct-ai",
        app_files={"server.py": b"import openai\n"},
    )
    with pytest.raises(ValueError, match="direct model-provider SDKs.*openai"):
        store_submission.review_deb(direct_sdk_deb)


def test_approve_strips_untrusted_provenance_signs_payload_and_passes_validate_deb(tmp_path):
    submitted_deb = _pack_submission_deb(
        tmp_path / "incoming",
        app_files={".provenance.json": b'{"schema":"untrusted-developer-envelope"}\n'},
    )
    key_path = tmp_path / "publisher-key.json"
    key_info = store_submission.generate_signing_key(key_path, comment="Store owner test key")

    approved_deb = tmp_path / "approved/yuos-app-hello-notes_1.0.0_all.deb"
    result = store_submission.approve_deb(submitted_deb, approved_deb, key_path)

    assert result["approved"] is True
    assert result["app_id"] == "hello-notes"
    assert result["provenance"]["publisher_key_id"] == key_info["key_id"]
    assert approved_deb.is_file()

    with store_submission._posix_scratch(prefix=".test-verify-") as scratch:
        validate_deb(approved_deb, environment=command_environment(), scratch=scratch)
        extracted = scratch / "extracted"
        run(["dpkg-deb", "--raw-extract", approved_deb, extracted])
        app_root = extracted / "usr/lib/yuos/apps/hello-notes"
        envelope = store_submission.verify_provenance_envelope(
            app_root, expected_key_id=key_info["key_id"]
        )
        assert envelope["schema"] == "yuos.provenance/v1"
        assert envelope["signature"]["key_id"] == key_info["key_id"]
        assert set(envelope["package"]["entrypoints"]) == {"main.py", "server.py"}
        sums = (extracted / "DEBIAN/md5sums").read_text(encoding="utf-8")
        assert "usr/lib/yuos/apps/hello-notes/.provenance.json" in sums
