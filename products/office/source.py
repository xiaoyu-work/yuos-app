"""Prepare the exact LibreOffice source and ordered Yu Office patches."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile
from urllib.parse import urlsplit


PRODUCT = Path(__file__).resolve().parent
ROOT = PRODUCT.parents[1]
CACHE = ROOT / "build" / "office-upstream"


def inputs():
    pin = json.loads((PRODUCT / "upstream.json").read_text())
    if set(pin) != {"version", "url", "sha256", "size"} or not (
        re.fullmatch(r"\d+\.\d+\.\d+\.\d+", pin["version"])
        and re.fullmatch(r"[a-f0-9]{64}", pin["sha256"])
        and type(pin["size"]) is int and pin["size"] > 0
    ):
        raise ValueError("Invalid LibreOffice source lock")
    url = urlsplit(pin["url"])
    if url.scheme != "https" or not url.hostname or url.username or url.password or (
        url.path.rsplit("/", 1)[-1] != f"libreoffice-{pin['version']}.tar.xz"
    ):
        raise ValueError("LibreOffice source lock requires a versioned HTTPS archive")
    root = PRODUCT / "patches"
    names = (root / "series").read_text().splitlines()
    if not names or len(names) != len(set(names)) or any(
        not re.fullmatch(r"[0-9]{4}-[a-z0-9-]+\.patch", name) for name in names
    ):
        raise ValueError("List each Office patch once in patches/series, in application order")
    patches = [root / name for name in names]
    if set(patches) != set(root.glob("*.patch")) or any(
        not path.is_file() or path.is_symlink() for path in patches
    ):
        raise ValueError("Office patch series and checked-in patch files differ")
    return pin, patches


def verify_archive(path, pin):
    if not path.is_file() or path.is_symlink() or path.stat().st_size != pin["size"]:
        raise ValueError(f"LibreOffice source length does not match its lock: {path}")
    with path.open("rb") as stream:
        if hashlib.file_digest(stream, "sha256").hexdigest() != pin["sha256"]:
            raise ValueError(f"LibreOffice source SHA256 does not match its lock: {path}")


def archive_path(pin, archive=None):
    if archive is not None:
        verify_archive(archive, pin)
        return archive
    CACHE.mkdir(parents=True, exist_ok=True)
    path = CACHE / f"libreoffice-{pin['version']}.tar.xz"
    if not path.exists() and not path.is_symlink():
        with tempfile.TemporaryDirectory(prefix="office-download-", dir=CACHE) as work:
            download = Path(work) / path.name
            subprocess.run([
                "curl", "--fail", "--silent", "--show-error", "--location",
                "--proto", "=https", "--proto-redir", "=https", "--max-filesize", str(pin["size"]),
                "--output", str(download), pin["url"],
            ], check=True)
            verify_archive(download, pin)
            download.replace(path)
    verify_archive(path, pin)
    return path


def apply_series(source, patches):
    # A nested build directory must not make git apply use the App checkout as its root.
    environment = {**os.environ, "GIT_CEILING_DIRECTORIES": str(source.parent.resolve())}
    for patch in patches:
        print(f"Applying {patch.name}", flush=True)
        subprocess.run(["git", "apply", "--whitespace=error-all", "-"],
                       input=patch.read_bytes().replace(b"\r\n", b"\n"),
                       cwd=source, env=environment, check=True)


def prepare(output, *, archive=None):
    pin, patches = inputs()
    if output.exists() or output.is_symlink():
        raise ValueError(f"Refusing to overwrite Office source: {output}")
    path = archive_path(pin, archive)
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="office-source-", dir=output.parent) as work:
        with tarfile.open(path, "r:xz") as source:
            source.extractall(path=work, filter="data")
        checkout = Path(work) / f"libreoffice-{pin['version']}"
        if not (checkout / "COPYING").is_file() or not (checkout / "configure.ac").is_file():
            raise ValueError("LibreOffice source archive has an unexpected layout")
        apply_series(checkout, patches)
        checkout.rename(output)
    return pin


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["check", "prepare"])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--archive", type=Path, help="Use an existing archive only if it matches the lock")
    args = parser.parse_args()
    if args.command == "check":
        if args.output:
            parser.error("check does not stage a source tree")
        pin, patches = inputs()
        if args.archive:
            verify_archive(args.archive, pin)
        print(f"LibreOffice {pin['version']}; {len(patches)} ordered patch(es)")
    else:
        if not args.output:
            parser.error("prepare requires a new --output directory")
        prepare(args.output.resolve(), archive=args.archive)


if __name__ == "__main__":
    main()
