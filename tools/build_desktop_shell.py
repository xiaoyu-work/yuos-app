"""Build the unsigned Desktop Shell input to yuos-desktop."""

import argparse
from pathlib import Path
import subprocess
import sys
import tempfile

import native_payload
import release_snapshots as snapshots
from release import scan_native_payload
from release_common import ROOT, command_environment, run, write_json


WORK = ROOT / "build/desktop-shell"
APP = "desktop-shell"
BUILD_FORMAT = "yuos.desktop-shell-build/v1"


def build(architecture):
    if "VERSION_CODENAME=trixie" not in Path("/etc/os-release").read_text().splitlines() or (
        run(["dpkg", "--print-architecture"]).stdout.decode().strip() != architecture
    ):
        raise ValueError("Desktop Shell must be built natively on the selected Debian 13 architecture")
    revision = run(["git", "rev-parse", "HEAD"]).stdout.decode().strip()
    WORK.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="build-", dir=WORK) as scratch:
        root = Path(scratch) / APP
        subprocess.run([
            sys.executable, ROOT / "tools/native_build.py", APP, "build", "--release", "--app-root", root,
        ], check=True, cwd=ROOT, env=command_environment())
        selected = native_payload.plan(APP)
        # Native staging preserves the checked-in App manifest byte-for-byte.
        version = snapshots.read_metadata(root / "app.json")["version"]
        dependencies = scan_native_payload(root, "yuos-desktop", architecture, root / "bin", [
            root / "resources" / path for path in selected.private_resources
        ])
        write_json(root / "build.json", {
            "format": BUILD_FORMAT, "architecture": architecture, "release_version": version,
            "source_revision": revision, "depends": dependencies,
        })
        snapshots.write_archive(root, snapshots.tree_inventory(root, architecture),
                                WORK / f"unsigned-{architecture}.tar.gz")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--architecture", choices=["amd64", "arm64"], required=True)
    args = parser.parse_args()
    build(args.architecture)


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        detail = error.stderr or error.stdout
        raise SystemExit(detail.decode(errors="replace").strip() if detail else str(error))
    except (ValueError, KeyError, OSError) as error:
        raise SystemExit(f"Desktop Shell build failed: {error}")
