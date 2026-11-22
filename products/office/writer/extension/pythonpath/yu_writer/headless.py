"""Private headless Writer sessions for the MCP surface.

Each call starts Yu Office's own runtime with a temporary profile, loads a
private scratch copy without macros or link updates, and exports a scratch
result. The process never connects to the owner's GUI instance or user path.
"""

import os
from pathlib import Path
import secrets
import shutil
import subprocess
import sys
import tempfile
import time

from .catalog import ToolError
from .tools import Target

FILTERS = {".docx": "MS Word 2007 XML", ".odt": "writer8"}
START_TIMEOUT = 45
STOP_TIMEOUT = 15


def import_uno(program_dir):
    """Import pyuno from the private runtime before any other installation."""
    program_dir = str(Path(program_dir).resolve())
    if "uno" in sys.modules:
        return sys.modules["uno"]
    os.environ.setdefault("URE_BOOTSTRAP", f"vnd.sun.star.pathname:{program_dir}/fundamentalrc")
    if program_dir not in sys.path:
        sys.path.insert(0, program_dir)
    import uno

    return uno


def _properties(uno, **values):
    result = []
    for name, value in values.items():
        prop = uno.createUnoStruct("com.sun.star.beans.PropertyValue")
        prop.Name, prop.Value = name, value
        result.append(prop)
    return tuple(result)


class HeadlessOffice:
    def __init__(self, program_dir):
        self.program_dir = Path(program_dir)
        self.process = None
        self.desktop = None

    def __enter__(self):
        self.uno = import_uno(self.program_dir)
        self.workspace = Path(tempfile.mkdtemp(prefix="yu-writer-"))
        os.chmod(self.workspace, 0o700)
        pipe = f"yu-writer-{secrets.token_hex(12)}"
        profile = (self.workspace / "profile").as_uri()
        environment = {key: value for key, value in os.environ.items()
                       if key not in {"DISPLAY", "WAYLAND_DISPLAY", "SESSION_MANAGER"}}
        environment.update({"SAL_USE_VCLPLUGIN": "svp", "HOME": str(self.workspace),
                            "TMPDIR": str(self.workspace)})
        try:
            self._launch(pipe, profile, environment)
            self.desktop = self._connect(pipe, lambda: self._launch(pipe, profile, environment))
        except BaseException:
            self.__exit__(None, None, None)
            raise
        return self

    def _launch(self, pipe, profile, environment):
        self.process = subprocess.Popen(
            [str(self.program_dir / "soffice.bin"), f"-env:UserInstallation={profile}",
             "--headless", "--invisible", "--norestore", "--nologo", "--nodefault",
             "--nolockcheck", f"--accept=pipe,name={pipe};urp;StarOffice.ComponentContext"],
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            env=environment, cwd=self.workspace, start_new_session=True,
        )

    def _connect(self, pipe, relaunch):
        local = self.uno.getComponentContext()
        resolver = local.ServiceManager.createInstanceWithContext(
            "com.sun.star.bridge.UnoUrlResolver", local)
        deadline = time.monotonic() + START_TIMEOUT
        restarted = False
        while True:
            try:
                remote = resolver.resolve(f"uno:pipe,name={pipe};urp;StarOffice.ComponentContext")
                self.context = remote
                return remote.ServiceManager.createInstanceWithContext("com.sun.star.frame.Desktop", remote)
            except Exception:  # noqa: BLE001 - NoConnectException until soffice listens
                status = self.process.poll()
                # 81 asks for one restart after initializing the new private profile.
                if status == 81 and not restarted:
                    restarted = True
                    relaunch()
                    continue
                if status is not None:
                    raise ToolError("office_unavailable", "Yu Office could not start its document engine.")
                if time.monotonic() > deadline:
                    raise ToolError("office_unavailable", "Yu Office's document engine did not start in time.")
                time.sleep(0.1)

    def load(self, data, suffix):
        source = self.workspace / f"input{suffix}"
        source.write_bytes(data)
        try:
            document = self.desktop.loadComponentFromURL(
                source.as_uri(), "_blank", 0,
                _properties(self.uno, Hidden=True, ReadOnly=False, MacroExecutionMode=0,
                            UpdateDocMode=0, Silent=True))
        except Exception as error:  # noqa: BLE001 - UNO IO/IllegalArgument exceptions
            raise ToolError("unreadable_document", f"Writer could not open the document: {error}") from None
        if document is None or not document.supportsService("com.sun.star.text.TextDocument"):
            if document is not None:
                document.close(True)
            raise ToolError("unsupported_document", "The file is not a Writer text document.")
        return document

    def export(self, document, suffix):
        target = self.workspace / f"output{suffix}"
        try:
            document.storeToURL(target.as_uri(), _properties(self.uno, FilterName=FILTERS[suffix], Overwrite=True))
        except Exception as error:  # noqa: BLE001
            raise ToolError("save_failed", f"Writer could not export the document: {error}") from None
        return target.read_bytes()

    def __exit__(self, *_):
        if self.desktop is not None:
            try:
                components = self.desktop.getComponents().createEnumeration()
                while components.hasMoreElements():
                    components.nextElement().close(True)
                self.desktop.terminate()
            except Exception:  # noqa: BLE001 - the bridge closes during terminate
                pass
            self.desktop = None
        if self.process is not None:
            try:
                self.process.wait(STOP_TIMEOUT)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(STOP_TIMEOUT)
            self.process = None
        if getattr(self, "workspace", None) is not None:
            shutil.rmtree(self.workspace, ignore_errors=True)
            self.workspace = None


class FileTarget(Target):
    """A private copy of one file obtained and committed by an OS file adapter.

    `source` must provide `suffix`, `read() -> bytes` and `commit(bytes) -> dict`.
    The adapter alone establishes authority for the target.
    """

    def __init__(self, source, office):
        self.source = source
        self.office = office
        self._document = None

    def document(self):
        if self._document is None:
            self._document = self.office.load(self.source.read(), self.source.suffix)
        return self._document

    def commit(self, document):
        return self.source.commit(self.office.export(document, self.source.suffix))
