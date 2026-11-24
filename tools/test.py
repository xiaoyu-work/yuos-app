"""Run declared App tests with verified platform artifact and local library exports."""

import argparse
import json
import os
import subprocess
import sys

from pathlib import Path
from stage import ROOT, app_entries, load_package, python_libraries, shared_python_root


def source_tests(source, package):
    declared = [source / path for path in package["tests"]]
    automatic = []
    for app in app_entries(source, package).values():
        default = app / "test_main.py"
        manifest = json.loads((app / "app.json").read_text())
        native_gui_only = (
            package.get("native_kind") == "binary"
            and manifest.get("runtime") == "binary"
            and isinstance(manifest.get("desktop"), dict)
            and "mcp" not in manifest
            and not manifest.get("operations")
        )
        if default.is_file():
            automatic.append(default)
        elif not native_gui_only and not any(
            path.is_file() and path.resolve().is_relative_to(app.resolve()) for path in declared
        ):
            raise ValueError(f"App without test_main.py requires an explicit test file: {app}")
    return list(dict.fromkeys([*automatic, *declared]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("product", nargs="*")
    parser.add_argument("--capability", nargs="+", default=[])
    parser.add_argument("--shared", action="store_true", help="Run shared App library contracts")
    options = parser.parse_args()
    if not options.product and not options.capability and not options.shared:
        parser.error("select a product, --capability source group, or --shared")
    packages = [
        *(load_package(name) for name in options.product),
        *(load_package(name, "capability") for name in options.capability),
    ]
    paths = [ROOT / "yuos-sdk/python/src", shared_python_root(ROOT)]
    env = os.environ.copy()
    env["PYTHONDONTWRITEBYTECODE"] = "1"
    tests = []
    for product, package in packages:
        apps = app_entries(product, package)
        paths.extend(path.parent for path in python_libraries(product, package, list(apps)).values())
        tests.extend(source_tests(product, package))
    env["PYTHONPATH"] = os.pathsep.join(map(str, dict.fromkeys([ROOT / "tests", *paths])))
    tests.extend([
        ROOT / "tests" / "shared",
        ROOT / "tests" / "test_manifest_contracts.py",
        ROOT / "tests" / "test_stage.py",
    ])
    subprocess.run([sys.executable, "-m", "pytest", "-q", "--import-mode=importlib",
                    *map(str, tests)],
                   cwd=ROOT, env=env, check=True)


if __name__ == "__main__":
    main()
