"""Compile a real Yu Office runtime from the verified, patched source."""

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

NATIVE = Path(__file__).resolve().parent
sys.path.insert(0, str(NATIVE.parent))
from source import prepare

MAX_RUNTIME_BYTES = 512 * 1024 * 1024


def configure(source, workspace, jobs):
    branding = workspace / "branding"
    branding.mkdir()
    icon = NATIVE / "com.yuos.Office.svg"
    for name in ("logo.svg", "about.svg"):
        shutil.copyfile(icon, branding / name)
    subprocess.run(["rsvg-convert", "--output", str(branding / "intro.png"), str(icon)], check=True)
    environment = {
        **os.environ,
        "GIT_CEILING_DIRECTORIES": str(source.parent.resolve()),
        "CFLAGS": "-Os -g0",
        "CXXFLAGS": "-Os -g0",
    }
    if Path("/.dockerenv").is_file():
        environment.setdefault("container", "docker")
    subprocess.run([
        "sh", "./autogen.sh", "--enable-release-build", "--enable-mergelibs",
        "--enable-gtk3", "--disable-gtk4", "--disable-qt5", "--disable-qt6",
        "--disable-gtk3-kde5", "--disable-kf5", "--disable-kf6", "--disable-skia",
        "--disable-odk", "--disable-online-update", "--without-help", "--without-java",
        "--without-myspell-dicts", "--without-fonts", "--without-doxygen",
        "--with-system-icu", "--with-system-curl", "--with-theme=colibre colibre_dark",
        "--enable-python=system", "--with-lang=en-US", "--with-vendor=Yu OS",
        f"--with-branding={branding}", f"--with-parallelism={jobs}",
    ], cwd=source, env=environment, check=True)
    return environment


def build(output, jobs):
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="office-build-", dir=output.parent) as temporary:
        workspace = Path(temporary)
        source = workspace / "source"
        pin = prepare(source)
        environment = configure(source, workspace, jobs)
        subprocess.run(["make", "build"], cwd=source, env=environment, check=True)
        runtime = source / "instdir"
        program = runtime / "program"
        bootstrap = (program / "bootstraprc").read_text()
        if "ProductKey=Yu Office " not in bootstrap or "UserInstallation=$SYSUSERCONFIG/yu%20office/4" not in bootstrap:
            raise ValueError("Office build did not preserve its independent product and profile identity")
        if not os.access(program / "soffice.bin", os.X_OK):
            raise ValueError("Patched Office build did not produce its real executable")
        shutil.copyfile(source / "COPYING", runtime / "UPSTREAM-COPYING")
        for extra_dir in ("sdk", "share/gallery", "share/wizards", "share/template", "share/autotext"):
            candidate = runtime / extra_dir
            if candidate.exists() and not candidate.is_symlink():
                shutil.rmtree(candidate)
        for svg_zip in (runtime / "share/config").glob("images_*_svg.zip"):
            svg_zip.unlink()
        # The signed App format requires owned files rather than symlinks.
        for path in runtime.rglob("*"):
            if path.is_symlink():
                target = path.resolve(strict=True)
                if not target.is_file() or not target.is_relative_to(runtime):
                    raise ValueError(f"Office runtime contains an unsupported link: {path}")
                path.unlink()
                shutil.copyfile(target, path)
        size = 0
        for path in runtime.rglob("*"):
            if path.is_file():
                with path.open("rb") as stream:
                    elf = stream.read(4) == b"\x7fELF"
                if elf:
                    subprocess.run(["strip", "--strip-unneeded", str(path)], check=True)
                size += path.stat().st_size
        if size > MAX_RUNTIME_BYTES:
            raise ValueError(f"Patched Office runtime exceeds the App signature size budget: {size} bytes")
        profile = (workspace / "smoke-profile").as_uri()
        version = subprocess.check_output([
            program / "soffice.bin", f"-env:UserInstallation={profile}", "--headless", "--version",
        ], cwd=workspace, env=environment, text=True, timeout=45)
        if not version.startswith(f"Yu Office {pin['version']}"):
            raise ValueError(f"Unexpected patched Office version: {version.strip()}")
        if output.is_symlink() or (output.exists() and not output.is_dir()):
            raise ValueError(f"Invalid Office output: {output}")
        if output.is_dir():
            shutil.rmtree(output)
        runtime.rename(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--jobs", type=int, required=True)
    args = parser.parse_args()
    if args.jobs < 1 or args.output.name != "office-runtime":
        parser.error("Office build requires positive jobs and an office-runtime output directory")
    if args.output.is_symlink():
        parser.error("Office runtime output must not be a symlink")
    build(args.output.absolute(), args.jobs)


if __name__ == "__main__":
    main()
