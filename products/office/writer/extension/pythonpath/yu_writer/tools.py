"""One dispatcher for the Writer tools used by the System Agent and in-Office AI.

A surface supplies only a `Target`: the live GUI document or a private
headless copy of an OS-authorized file. Tool names, arguments, validation and
handlers are identical on both surfaces.
"""

from contextlib import contextmanager

from . import document as model
from .catalog import Catalog, ToolError

INSPECT = "yu-office.writer.inspect"
PREVIEW = "yu-office.writer.preview"
APPLY = "yu-office.writer.apply"
UNDO_TITLE = "Yu Writer AI"


class Target:
    """Document binding chosen by the surface adapter, never by a model."""

    bound = False  # True when the surface binds its own document instead of `path`.

    def document(self):
        raise NotImplementedError

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

    def document(self):
        return self._document

    @contextmanager
    def edit(self, document):
        if document.isReadonly():
            raise ToolError("read_only", "The document is read-only.")
        undo = document.getUndoManager()
        undo.enterUndoContext(UNDO_TITLE)
        document.lockControllers()
        completed = False
        try:
            yield
            completed = True
        finally:
            document.unlockControllers()
            undo.leaveUndoContext()
            if not completed and undo.isUndoPossible() and undo.getCurrentUndoActionTitle() == UNDO_TITLE:
                undo.undo()

    def commit(self, document):
        return {"saved": False, "undo": {"kind": "writer-undo", "title": UNDO_TITLE, "steps": 1}}


def _inspect(target, args):
    return model.inspect(target.document(), args["start"], args["limit"])


def _preview(target, args):
    document = target.document()
    _, current, changes, warnings = model.plan(document, args)
    return {"revision": current, "action": args["action"], "changes": changes,
            "warnings": warnings, "applied": False}


def _apply(target, args):
    document = target.document()
    elements, current, changes, warnings = model.plan(document, args)
    with target.edit(document):
        model.apply_changes(document, elements, args)
    result = {"previous_revision": current, "revision": model.revision(document),
              "action": args["action"], "changes": changes, "warnings": warnings, "applied": True}
    result.update(target.commit(document))
    return result


HANDLERS = {INSPECT: _inspect, PREVIEW: _preview, APPLY: _apply}


def check_catalog(catalog):
    if set(catalog.tools) != set(HANDLERS):
        raise ToolError("catalog_invalid", "Writer manifest tools and handlers differ.")
    return catalog


def call(catalog, name, arguments, target):
    """Validate against the manifest catalog and run the shared handler."""
    check_catalog(catalog)
    args = catalog.validate(name, arguments, bound_target=target.bound)
    return HANDLERS[name](target, args)


__all__ = ["APPLY", "INSPECT", "PREVIEW", "Catalog", "LiveDocumentTarget", "Target", "ToolError",
           "call", "check_catalog", "HANDLERS"]
