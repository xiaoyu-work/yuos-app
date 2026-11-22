"""Yu Writer Assistant: the in-Office surface for the shared Writer tools.

The dialog binds the live Writer document (including unsaved text) and calls
the same catalog-validated handlers as the System Agent MCP tools. Manual
formatting works without AI; the optional AI path only produces a validated
preview, and the user must explicitly apply it.
"""

import json

import unohelper
from com.sun.star.awt import XActionListener
from com.sun.star.frame import XDispatch, XDispatchProvider
from com.sun.star.lang import XInitialization, XServiceInfo

from yu_writer import assistant
from yu_writer.catalog import Catalog, ToolError
from yu_writer.document import snapshot
from yu_writer.tools import APPLY, INSPECT, PREVIEW, UNDO_TITLE, LiveDocumentTarget, call

IMPLEMENTATION = "org.yuos.office.WriterAssistant"
PROTOCOL = "vnd.yuos.writer:"
UNCHANGED = "(unchanged)"
SWITCH = (UNCHANGED, "on", "off")
SCOPES = ("Selected paragraphs", "Whole document")
MAX_DIALOG_PREVIEW = 60_000


def _selected_indices(document, descriptions, elements):
    """Map the current body selection to paragraph block indices."""
    selection = document.getCurrentController().getSelection()
    if selection is None or not selection.supportsService("com.sun.star.text.TextRanges") or not selection.getCount():
        raise ToolError("invalid_target", "Place the cursor or a selection in body paragraphs.")
    chosen = selection.getByIndex(0)
    text = document.getText()
    indices = []
    try:
        for description, element in zip(descriptions, elements):
            if description["kind"] not in {"paragraph", "heading"}:
                continue
            if text.compareRegionStarts(chosen.getStart(), element.getEnd()) == -1:
                continue
            if text.compareRegionStarts(element.getStart(), chosen.getEnd()) == -1:
                break
            indices.append(description["index"])
    except Exception:  # noqa: BLE001 - IllegalArgumentException outside the body text
        raise ToolError("invalid_target", "The selection is not in body paragraphs.") from None
    if not indices:
        raise ToolError("invalid_target", "The selection contains no editable paragraphs.")
    return indices


class _Listener(unohelper.Base, XActionListener):
    def __init__(self, callback):
        self.callback = callback

    def actionPerformed(self, event):
        self.callback()

    def disposing(self, event):
        pass


class AssistantDialog:
    def __init__(self, context, frame):
        self.context = context
        self.document = frame.getController().getModel()
        self.target = LiveDocumentTarget(self.document)
        self.catalog = Catalog.load()
        self.pending = None
        self.allow_undo = True
        self.y = 6

    def _add(self, kind, name, height, label=None, x=6, width=248, **properties):
        model = self.model.createInstance(f"com.sun.star.awt.UnoControl{kind}Model")
        model.PositionX, model.PositionY, model.Width, model.Height = x, self.y, width, height
        if label is not None:
            model.Label = label
        for key, value in properties.items():
            setattr(model, key, value)
        self.model.insertByName(name, model)
        return model

    def _row(self, height=14):
        self.y += height + 4

    def _list(self, name, label, items):
        self._add("FixedText", f"{name}_label", 12, label, width=90)
        self._add("ListBox", name, 12, x=100, width=154, Dropdown=True,
                  StringItemList=tuple(items), SelectedItems=(0,))
        self._row(12)

    def _build(self):
        self.model = self.context.ServiceManager.createInstanceWithContext(
            "com.sun.star.awt.UnoControlDialogModel", self.context)
        self.model.Title = "Yu Writer Assistant"
        state = call(self.catalog, INSPECT, {"start": 0, "limit": 1}, self.target)
        styles = [style["name"] for style in state["paragraph_styles"] if style["in_use"]] or \
                 [style["name"] for style in state["paragraph_styles"]]
        self._list("scope", "Apply to", SCOPES)
        self._add("FixedText", "instruction_label", 12, "Ask Yu OS AI (optional)")
        self._row(12)
        self._add("Edit", "instruction", 36, MultiLine=True, VScroll=True)
        self._row(36)
        self._add("Button", "ask", 14, "Ask AI to preview a workflow", width=160)
        self._row()
        self._add("FixedLine", "format_line", 8, "Formatting")
        self._row(8)
        self._list("paragraph_style", "Paragraph style", (UNCHANGED, *styles))
        self._list("alignment", "Alignment", (UNCHANGED, *self.catalog.choices(PREVIEW, "alignment")))
        for name in ("bold", "italic", "underline"):
            self._list(name, name.capitalize(), SWITCH)
        self._add("FixedText", "font_name_label", 12, "Font name", width=90)
        self._add("Edit", "font_name", 12, x=100, width=154)
        self._row(12)
        self._add("FixedText", "font_size_label", 12, "Font size (pt)", width=90)
        self._add("Edit", "font_size", 12, x=100, width=154)
        self._row(12)
        self._add("Button", "preview_format", 14, "Preview formatting", width=120)
        self._row()
        self._add("Edit", "preview", 96, MultiLine=True, ReadOnly=True, VScroll=True)
        self._row(96)
        self._add("Button", "apply", 14, "Apply", width=60, Enabled=False)
        self._add("Button", "undo", 14, "Undo", x=72, width=60)
        self._add("Button", "close", 14, "Close", x=194, width=60, PushButtonType=2)
        self._row()
        self.model.Width, self.model.Height = 260, self.y
        self.dialog = self.context.ServiceManager.createInstanceWithContext(
            "com.sun.star.awt.UnoControlDialog", self.context)
        self.dialog.setModel(self.model)
        for name, callback in (("ask", self.ask), ("preview_format", self.preview_format),
                               ("apply", self.apply), ("undo", self.undo)):
            self.dialog.getControl(name).addActionListener(_Listener(self._guard(callback)))

    def _guard(self, callback):
        def run():
            try:
                callback()
            except ToolError as error:
                if error.code == "rollback_unconfirmed":
                    self.allow_undo = False
                self._show(str(error))
            except Exception as error:  # noqa: BLE001 - keep Writer usable on any failure
                self._show(f"Writer operation failed: {error}")
        return run

    def _show(self, text, pending=None):
        self.pending = pending
        self.dialog.getControl("preview").setText(text)
        self.model.getByName("apply").Enabled = pending is not None
        manager = self.document.getUndoManager()
        self.model.getByName("undo").Enabled = (
            self.allow_undo and manager.isUndoPossible() and manager.getCurrentUndoActionTitle() == UNDO_TITLE
        )

    def _choice(self, name):
        position = self.dialog.getControl(name).getSelectedItemPos()
        return None if position <= 0 else self.dialog.getControl(name).getItem(position)

    def _scope(self):
        if self.dialog.getControl("scope").getSelectedItemPos() == 1:
            return None
        elements, descriptions, _ = snapshot(self.document)
        return _selected_indices(self.document, descriptions, elements)

    def _all_paragraphs(self):
        state = call(self.catalog, INSPECT, {"start": 0, "limit": 500}, self.target)
        if state["next_start"] is not None or state["truncated"]:
            raise ToolError("scope_too_large", "The document is too long; select the paragraphs to change.")
        return state["revision"], [block["index"] for block in state["blocks"]
                                   if block["kind"] in {"paragraph", "heading"}]

    def preview_format(self):
        arguments = {"action": "format"}
        for name in ("paragraph_style", "alignment"):
            if self._choice(name):
                arguments[name] = self._choice(name)
        for name in ("bold", "italic", "underline"):
            if self._choice(name):
                arguments[name] = self._choice(name) == "on"
        font_name = self.dialog.getControl("font_name").getText().strip()
        if font_name:
            arguments["font_name"] = font_name
        font_size = self.dialog.getControl("font_size").getText().strip()
        if font_size:
            try:
                arguments["font_size"] = float(font_size)
            except ValueError:
                raise ToolError("invalid_arguments", "Font size must be a number.") from None
        revision, every = self._all_paragraphs()
        arguments["revision"] = revision
        arguments["paragraphs"] = self._scope() or every
        preview = call(self.catalog, PREVIEW, arguments, self.target)
        self._show(self._describe(preview), arguments)

    def ask(self):
        instruction = self.dialog.getControl("instruction").getText()
        self._show("Asking Yu OS AI…")
        result = assistant.propose(self.catalog, self.target, instruction, self._scope())
        text = self._describe(result["preview"])
        if result["explanation"]:
            text = f"AI: {result['explanation']}\n\n{text}"
        if len(text.encode("utf-16-le")) // 2 > MAX_DIALOG_PREVIEW:
            raise ToolError("preview_too_large", "This preview cannot fit in the dialog; request a smaller workflow.")
        self._show(text)
        if self.dialog.getControl("preview").getText().replace("\r\n", "\n") != text.replace("\r\n", "\n"):
            raise ToolError("preview_too_large", "Writer did not display the full preview; request a smaller workflow.")
        self.pending = result["arguments"]
        self.model.getByName("apply").Enabled = True

    def apply(self):
        if self.pending is None:
            return
        result = assistant.confirm(self.catalog, self.target, self.pending)
        self.allow_undo = bool(result["undo"])
        if result["undo"]:
            suffix = " Use Undo or Ctrl+Z to revert."
        else:
            suffix = " This change is not fully covered by Writer Undo."
        count = len(result["steps"]) if "steps" in result else len(result["changes"])
        subject = "workflow step(s)" if "steps" in result else "change(s)"
        self._show(f"Applied {count} {subject}.{suffix}")

    def undo(self):
        manager = self.document.getUndoManager()
        if not self.allow_undo or not manager.isUndoPossible() or manager.getCurrentUndoActionTitle() != UNDO_TITLE:
            raise ToolError("nothing_to_undo", "The last Writer action was not a Yu Writer Assistant change.")
        manager.undo()
        self._show("Undid the last Yu Writer Assistant change.")

    @staticmethod
    def _describe(preview, *, complete=False):
        if "steps" in preview:
            return "\n\n".join(f"{step['step']}. {AssistantDialog._describe(step, complete=True)}"
                               for step in preview["steps"])
        lines = [f"Preview: {preview['action']} ({len(preview['changes'])} change(s)):"]
        for change in preview["changes"] if complete else preview["changes"][:50]:
            if "paragraph" not in change:
                lines.append(f"\n{change['target']}: {json.dumps(change['before'], ensure_ascii=False)}"
                             f"\n    → {json.dumps(change['after'], ensure_ascii=False)}")
            elif preview["action"] in {"rewrite", "find_replace"}:
                lines.append(f"\n[{change['paragraph']}] before: {change['before']}\n    after: {change['after']}")
            else:
                lines.append(f"\n[{change['paragraph']}] {change['text'][:80]}\n    set: "
                             + json.dumps(change["after"], ensure_ascii=False))
        if not complete and len(preview["changes"]) > 50:
            lines.append(f"\n… {len(preview['changes']) - 50} more")
        lines.extend(f"\nWarning: {warning}" for warning in preview["warnings"])
        return "\n".join(lines)

    def run(self):
        self._build()
        toolkit = self.context.ServiceManager.createInstanceWithContext("com.sun.star.awt.Toolkit", self.context)
        self.dialog.createPeer(toolkit, None)
        try:
            self.dialog.execute()
        finally:
            self.dialog.dispose()


class WriterAssistant(unohelper.Base, XDispatchProvider, XDispatch, XInitialization, XServiceInfo):
    def __init__(self, context, *arguments):
        self.context = context
        self.frame = None

    def initialize(self, arguments):
        if arguments:
            self.frame = arguments[0]

    def queryDispatch(self, url, target, flags):
        return self if url.Protocol == PROTOCOL and url.Path == "assistant" else None

    def queryDispatches(self, requests):
        return tuple(self.queryDispatch(r.FeatureURL, r.FrameName, r.SearchFlags) for r in requests)

    def dispatch(self, url, arguments):
        if self.frame is None:
            return
        model = self.frame.getController().getModel()
        if model is None or not model.supportsService("com.sun.star.text.TextDocument"):
            return
        AssistantDialog(self.context, self.frame).run()

    def addStatusListener(self, listener, url):
        pass

    def removeStatusListener(self, listener, url):
        pass

    def getImplementationName(self):
        return IMPLEMENTATION

    def supportsService(self, name):
        return name == "com.sun.star.frame.ProtocolHandler"

    def getSupportedServiceNames(self):
        return ("com.sun.star.frame.ProtocolHandler",)


g_ImplementationHelper = unohelper.ImplementationHelper()
g_ImplementationHelper.addImplementation(WriterAssistant, IMPLEMENTATION, ("com.sun.star.frame.ProtocolHandler",))
