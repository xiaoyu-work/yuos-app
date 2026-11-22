"""One dispatcher for the Writer tools used by the System Agent and in-Office AI.

A surface supplies only a `Target`: the live GUI document or a private
headless copy of an OS-authorized file. Tool names, arguments, validation and
handlers are identical on both surfaces.
"""

from contextlib import contextmanager
from pathlib import Path
import secrets
import tempfile

from . import document as model
from . import workflow
from .catalog import Catalog, ToolError

INSPECT = "yu-office.writer.inspect"
PREVIEW = "yu-office.writer.preview"
APPLY = "yu-office.writer.apply"
PREVIEW_WORKFLOW = "yu-office.writer.preview_workflow"
APPLY_WORKFLOW = "yu-office.writer.apply_workflow"
WRITE_TOOLS = frozenset({APPLY, APPLY_WORKFLOW})
UNDO_TITLE = "Yu Writer AI"


class Target:
    """Document binding chosen by the surface adapter, never by a model."""

    bound = False  # True when the surface binds its own document instead of `path`.
    workflow_identity = "file"

    def check_cancelled(self):
        pass

    def document(self):
        raise NotImplementedError

    @contextmanager
    def rehearse(self, document):
        # File targets already hold a private document model, never the owner's open document.
        yield document

    @contextmanager
    def edit(self, document):
        yield

    def commit(self, document):
        return {}


class LiveDocumentTarget(Target):
    """The current Writer document in the GUI, including unsaved text."""

    bound = True

    def __init__(self, model_document):
        if not model_document.supportsService("com.sun.star.text.TextDocument"):
            raise ToolError("unsupported_document", "The Yu Writer assistant works only in Writer documents.")
        self._document = model_document
        self._did_add_undo = False
        self.workflow_identity = secrets.token_hex(16)

    def document(self):
        return self._document

    @contextmanager
    def rehearse(self, document):
        from .headless import _properties
        import uno

        controller = document.getCurrentController()
        desktop = controller.getFrame().getCreator() if controller else None
        if desktop is None or not hasattr(desktop, "loadComponentFromURL"):
            raise ToolError("workflow_unavailable", "Writer cannot create a private workflow preview in this window.")
        with tempfile.TemporaryDirectory(prefix="yu-writer-workflow-") as directory:
            location = (Path(directory) / "preview.odt").as_uri()
            document.storeToURL(location, _properties(uno, FilterName="writer8"))
            clone = desktop.loadComponentFromURL(location, "_blank", 0,
                                                 _properties(uno, Hidden=True, Silent=True,
                                                             MacroExecutionMode=0, UpdateDocMode=0))
            if clone is None or clone == document:
                raise ToolError("workflow_unavailable", "Writer could not open the private workflow preview.")
            try:
                yield clone
            finally:
                clone.setModified(False)
                clone.close(True)

    @contextmanager
    def edit(self, document):
        if document.isReadonly():
            raise ToolError("read_only", "The document is read-only.")
        undo = document.getUndoManager()
        previous = tuple(undo.getAllUndoActionTitles())
        self._did_add_undo = False
        undo.enterUndoContext(UNDO_TITLE)
        document.lockControllers()
        completed = False
        try:
            yield
            completed = True
        finally:
            document.unlockControllers()
            undo.leaveUndoContext()
            current = tuple(undo.getAllUndoActionTitles())
            self._did_add_undo = current != previous and bool(current) and current[0] == UNDO_TITLE
            if not completed and self._did_add_undo:
                undo.undo()
                self._did_add_undo = False

    def commit(self, document):
        return {"saved": False, "undo": {"kind": "writer-undo", "title": UNDO_TITLE, "steps": 1}
                if self._did_add_undo else None}

    def restore_workflow(self, document, restores, revision, modified):
        undo = document.getUndoManager()
        undo.lock()
        try:
            for restore in reversed(restores):
                restore()
            if model.revision(document) != revision:
                raise ToolError("rollback_unconfirmed", "Writer could not verify that all live changes were undone.")
            document.setModified(modified)
        except Exception as error:
            raise ToolError("rollback_unconfirmed",
                            "Writer could not verify rollback; inspect the open document before saving.") from error
        finally:
            undo.unlock()


def _inspect(target, args):
    return model.inspect(target.document(), args["start"], args["limit"],
                         args.get("view", "body"), args.get("table_name"))


def _preview(target, args):
    document = target.document()
    _, current, changes, warnings = model.plan(document, args)
    return {"revision": current, "action": args["action"], "changes": changes,
            "warnings": warnings, "applied": False}


def _apply(target, args):
    document = target.document()
    edit, current, changes, warnings = model.plan(document, args)
    with target.edit(document):
        model.apply_changes(edit)
    result = {"previous_revision": current, "revision": model.revision(document),
              "action": args["action"], "changes": changes, "warnings": warnings, "applied": True}
    result.update(target.commit(document))
    return result


def _preview_workflow(catalog, target, args):
    return workflow.run(catalog, PREVIEW, target, args, apply=False)


def _apply_workflow(catalog, target, args):
    return workflow.run(catalog, PREVIEW, target, args, apply=True)


WORKFLOW_HANDLERS = {PREVIEW_WORKFLOW: _preview_workflow, APPLY_WORKFLOW: _apply_workflow}
HANDLERS = {INSPECT: _inspect, PREVIEW: _preview, APPLY: _apply, **WORKFLOW_HANDLERS}


def check_catalog(catalog):
    if set(catalog.tools) != set(HANDLERS):
        raise ToolError("catalog_invalid", "Writer manifest tools and handlers differ.")
    return catalog


def call(catalog, name, arguments, target):
    """Validate against the manifest catalog and run the shared handler."""
    check_catalog(catalog)
    args = catalog.validate(name, arguments, bound_target=target.bound)
    if name in WORKFLOW_HANDLERS:
        return HANDLERS[name](catalog, target, args)
    return HANDLERS[name](target, args)


__all__ = ["APPLY", "APPLY_WORKFLOW", "INSPECT", "PREVIEW", "PREVIEW_WORKFLOW", "WRITE_TOOLS",
           "Catalog", "LiveDocumentTarget", "Target", "ToolError", "call", "check_catalog", "HANDLERS"]
