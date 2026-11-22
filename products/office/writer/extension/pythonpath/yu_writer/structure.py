"""Bounded Writer operations for tables, layout, notes and review."""

import re
from urllib.parse import urlsplit

from .catalog import ToolError

MAX_RECORDS = 2_000
MAX_CELLS = 20_000
MAX_BATCH = 400
MAX_TEXT = 20_000
MAX_EDIT_TEXT = 128_000
PAGE_PROPERTIES = {
    "landscape": "IsLandscape", "page_width_mm": "Width", "page_height_mm": "Height",
    "left_margin_mm": "LeftMargin", "right_margin_mm": "RightMargin",
    "top_margin_mm": "TopMargin", "bottom_margin_mm": "BottomMargin",
}
BASE_ARGS = {"action", "revision", "path"}


def expect(args, required=(), optional=()):
    missing = set(required) - args.keys()
    extra = args.keys() - BASE_ARGS - set(required) - set(optional)
    if missing:
        raise ToolError("invalid_arguments", f"{args['action']} requires {', '.join(sorted(missing))}.")
    if extra:
        raise ToolError("invalid_arguments", f"{args['action']} does not accept {', '.join(sorted(extra))}.")


def paragraphs(values):
    if (len(values) > MAX_BATCH or sum(map(len, values)) > MAX_EDIT_TEXT
            or any(len(text) > MAX_TEXT or any(ch in text for ch in "\0\r\n\x0b\x0c\u2028\u2029")
                   for text in values)):
        raise ToolError("invalid_arguments", "Provide a bounded batch of single-paragraph texts.")


def _iterate(collection):
    enumeration = collection.createEnumeration()
    while enumeration.hasMoreElements():
        yield enumeration.nextElement()


def _plain(text):
    for paragraph in _iterate(text):
        if not paragraph.supportsService("com.sun.star.text.Paragraph"):
            return False
        if paragraph.createContentEnumeration("com.sun.star.text.TextContent").hasMoreElements():
            return False
        formats = set()
        for portion in _iterate(paragraph):
            if portion.TextPortionType != "Text" or portion.HyperLinkURL:
                return False
            formats.add((portion.CharFontName, portion.CharHeight, portion.CharWeight,
                         portion.CharPosture.value, portion.CharUnderline,
                         portion.CharColor, portion.CharStyleName))
        if len(formats) > 1:
            return False
    return True


def _one_plain(text):
    return "\n" not in text.getString() and _plain(text)


def _meta(record, target, before, after):
    return {"kind": record, "target": target, "before": before, "after": after}


def _restorable(apply, restore):
    def edit():
        try:
            apply()
        except Exception:
            restore()
            raise
    return edit


def _cells(table, limit=MAX_CELLS):
    names = table.getCellNames()
    if len(names) > limit:
        raise ToolError("document_too_large", f"A document may contain at most {MAX_CELLS} top-level table cells.")
    for name in names:
        yield name, table.getCellByName(name)


def table_fingerprint(table, remaining):
    return [(name, cell.getType().value, cell.getString(), cell.getFormula(), cell.IsProtected, _one_plain(cell))
            for name, cell in _cells(table, remaining)]


def _cell_records(table):
    for index, (name, cell) in enumerate(_cells(table)):
        kind = cell.getType().value
        yield {"index": index, "kind": "cell", "name": name, "text": cell.getString(),
               "value_kind": kind.lower(),
               "editable": kind in {"TEXT", "EMPTY"} and not cell.IsProtected and _one_plain(cell)}


def _page_values(style):
    return {key: bool(style.IsLandscape) if key == "landscape" else getattr(style, prop) / 100
            for key, prop in PAGE_PROPERTIES.items()}


def _region(style, region):
    prefix = region.capitalize()
    if not getattr(style, prefix + "IsOn"):
        return {"enabled": False, "shared": True, "text": "", "editable": True}
    shared = bool(getattr(style, prefix + "IsShared") and style.FirstIsShared)
    body = getattr(style, prefix + "Text")
    return {"enabled": True, "shared": shared, "text": body.getString(),
            "editable": shared and _one_plain(body)}


def _pages(document):
    family = document.getStyleFamilies().getByName("PageStyles")
    names = family.getElementNames()
    if len(names) > MAX_RECORDS:
        raise ToolError("document_too_large", "Too many page styles to inspect.")
    for index, name in enumerate(names):
        style = family.getByName(name)
        yield {"index": index, "kind": "page_style", "name": name, "display_name": style.DisplayName,
               "in_use": bool(style.isInUse()), **_page_values(style),
               "header": _region(style, "header"), "footer": _region(style, "footer")}


def _note_collection(document, kind):
    return document.getFootnotes() if kind == "footnote" else document.getEndnotes()


def _notes(document):
    for kind in ("footnote", "endnote"):
        collection = _note_collection(document, kind)
        if collection.getCount() > MAX_RECORDS:
            raise ToolError("document_too_large", "Too many notes to inspect.")
        for index in range(collection.getCount()):
            note = collection.getByIndex(index)
            text = note.getString()
            yield {"kind": kind, "index": index, "label": note.getLabel(),
                   "text": text, "editable": _one_plain(note)}


def _review(document):
    changes = _iterate(document.getRedlines())
    for index, change in enumerate(changes):
        if index >= MAX_RECORDS:
            raise ToolError("document_too_large", "Too many tracked changes to inspect.")
        yield {"kind": "tracked_change", "index": index, "change_type": change.RedlineType,
               "author": change.RedlineAuthor, "text": change.RedlineDescription,
               "comment": change.RedlineComment}
    index = 0
    for field in _iterate(document.getTextFields()):
        if not field.supportsService("com.sun.star.text.textfield.Annotation"):
            continue
        if index >= MAX_RECORDS:
            raise ToolError("document_too_large", "Too many comments to inspect.")
        yield {"kind": "comment", "index": index, "author": field.Author, "name": field.Name,
               "resolved": field.Resolved, "text": field.Content}
        index += 1


def capture(document):
    return {"layout": list(_pages(document)), "notes": list(_notes(document)),
            "review": list(_review(document)), "tracking": _tracking(document)}


def list_styles(document):
    return list(document.getStyleFamilies().getByName("NumberingStyles").getElementNames()[:500])


def view_records(document, details, descriptions, view, table_name):
    if table_name is not None and view != "tables":
        raise ToolError("invalid_arguments", "table_name is accepted only in the tables view.")
    if view == "body":
        return descriptions
    if view == "tables":
        if table_name is None:
            return [block for block in descriptions if block["kind"] == "table"]
        return list(_cell_records(_table(document, descriptions, table_name)))
    return details[view]


def _table(document, descriptions, name):
    if not any(block["kind"] == "table" and block["name"] == name for block in descriptions):
        raise ToolError("invalid_target", "Use a top-level table name from inspect.")
    tables = document.getTextTables()
    if not tables.hasByName(name):
        raise ToolError("invalid_target", "Use an existing table name from inspect.")
    return tables.getByName(name)


def _anchor(elements, descriptions, args):
    indices = args["paragraphs"]
    if len(indices) != 1 or not 0 <= indices[0] < len(elements):
        raise ToolError("invalid_target", "Select one body paragraph as the anchor.")
    index = indices[0]
    paragraph = descriptions[index]
    if paragraph["kind"] not in {"paragraph", "heading"} or not paragraph["rewritable"]:
        raise ToolError("unsupported_content", "Choose a body paragraph without protected content.")
    return index, elements[index]


def _column(index):
    result = ""
    while index >= 0:
        index, digit = divmod(index, 26)
        result = chr(65 + digit) + result
        index -= 1
    return result


def _insert_paragraphs(document, elements, descriptions, details, args):
    expect(args, ("paragraphs", "texts"), ("position",))
    index, anchor = _anchor(elements, descriptions, args)
    values, position = args["texts"], args.get("position", "after")
    if not values:
        raise ToolError("invalid_arguments", "Provide at least one paragraph to insert.")
    paragraphs(values)

    def apply():
        body = document.getText()
        start = anchor.getStart() if position == "before" else anchor.getEnd()
        cursor = body.createTextCursorByRange(start)
        if position == "after":
            body.insertControlCharacter(cursor, 0, False)
        for i, value in enumerate(values):
            body.insertString(cursor, value, False)
            if position == "before" or i < len(values) - 1:
                body.insertControlCharacter(cursor, 0, False)

    return apply, [_meta("insert_paragraphs", index, None, {"position": position, "texts": values})]


def _insert_table(document, elements, descriptions, details, args):
    expect(args, ("paragraphs", "rows", "columns"), ("table_name", "texts"))
    index, anchor = _anchor(elements, descriptions, args)
    rows, columns = args["rows"], args["columns"]
    values, name = args.get("texts", []), args.get("table_name")
    if not (1 <= rows <= MAX_BATCH and 1 <= columns <= 50 and rows * columns <= MAX_BATCH):
        raise ToolError("invalid_arguments", f"Create a table with at most {MAX_BATCH} cells and 50 columns.")
    if values and len(values) != rows * columns:
        raise ToolError("invalid_arguments", "Supply every cell in row-major order, or no cell values.")
    paragraphs(values)
    if name is not None and (not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]{0,63}", name)
                             or document.getTextTables().hasByName(name)):
        raise ToolError("invalid_arguments", "Provide a new table name using letters, digits or underscores.")

    def apply():
        table = document.createInstance("com.sun.star.text.TextTable")
        table.initialize(rows, columns)
        body = document.getText()
        body.insertTextContent(body.createTextCursorByRange(anchor.getEnd()), table, False)
        if name is not None:
            table.setName(name)
        for i, value in enumerate(values):
            table.getCellByName(f"{_column(i % columns)}{i // columns + 1}").setString(value)

    return apply, [_meta("insert_table", index, None,
                         {"name": name, "rows": rows, "columns": columns, "texts": values})]


def _set_cells(document, elements, descriptions, details, args):
    expect(args, ("table_name", "cells", "texts"))
    name, names, values = args["table_name"], args["cells"], args["texts"]
    table = _table(document, descriptions, name)
    if not names or len(names) != len(set(names)) or len(names) != len(values):
        raise ToolError("invalid_arguments", "Provide distinct exact cell names and one text per cell.")
    paragraphs(values)
    available = set(table.getCellNames())
    updates, changes = [], []
    for cell_name, value in zip(names, values):
        if cell_name not in available:
            raise ToolError("invalid_target", f"Cell {cell_name} does not exist in table {name}.")
        cell = table.getCellByName(cell_name)
        if cell.IsProtected or cell.getType().value not in {"TEXT", "EMPTY"} or not _one_plain(cell):
            raise ToolError("unsupported_content", f"Cell {cell_name} is protected or is not plain text.")
        updates.append((cell, value))
        changes.append(_meta("table_cell", f"{name}/{cell_name}", cell.getString(), value))

    def apply():
        for cell, value in updates:
            cell.setString(value)

    return apply, changes


def _table_axis(document, elements, descriptions, details, args):
    expect(args, ("table_name", "axis", "axis_action", "index", "count"))
    name, axis, operation = args["table_name"], args["axis"], args["axis_action"]
    index, count = args["index"], args["count"]
    table = _table(document, descriptions, name)
    rows, columns = table.getRows(), table.getColumns()
    height, width = rows.getCount(), columns.getCount()
    collection = rows if axis == "rows" else columns
    size = collection.getCount()
    if (not 1 <= count <= 50 or not 0 <= index <= size
            or operation == "remove_empty" and (index + count > size or size == count)):
        raise ToolError("invalid_arguments", "Invalid row or column range; keep at least one row and column.")
    if operation == "insert" and (size + count) * (width if axis == "rows" else height) > MAX_CELLS:
        raise ToolError("invalid_arguments", "The resulting table would exceed the cell limit.")
    if height * width > MAX_CELLS:
        raise ToolError("unsupported_content", "The table is too large to resize.")
    expected = {f"{_column(col)}{row + 1}" for row in range(height) for col in range(width)}
    if expected != set(table.getCellNames()):
        raise ToolError("unsupported_content", "Only rectangular tables without merged cells can be resized.")
    for row in range(height):
        for col in range(width):
            cell = table.getCellByName(f"{_column(col)}{row + 1}")
            kind = cell.getType().value
            removing = operation == "remove_empty" and index <= (row if axis == "rows" else col) < index + count
            if cell.IsProtected or kind == "FORMULA" or removing and (kind != "EMPTY" or not _one_plain(cell)):
                raise ToolError("unsupported_content", "Cannot resize a protected or formula table or remove nonempty cells.")
    apply = collection.insertByIndex if operation == "insert" else collection.removeByIndex
    return lambda: apply(index, count), [_meta("table_axis", name, {axis: size},
                                                {"axis": axis, "action": operation, "index": index, "count": count})]


def _page_style(document, name):
    family = document.getStyleFamilies().getByName("PageStyles")
    if not family.hasByName(name):
        raise ToolError("invalid_target", "Use an existing page style name from inspect layout.")
    return family.getByName(name)


def _setup(document, elements, descriptions, details, args):
    expect(args, ("page_style",), PAGE_PROPERTIES)
    updates = {key: args[key] for key in PAGE_PROPERTIES if key in args}
    if not updates:
        raise ToolError("invalid_arguments", "Provide at least one page setting.")
    name = args["page_style"]
    style = _page_style(document, name)
    before = _page_values(style)
    after = {**before, **updates}
    width, height = "page_width_mm", "page_height_mm"
    if ("landscape" in updates and width not in updates and height not in updates
            and before["landscape"] != after["landscape"]):
        after[width], after[height] = before[height], before[width]
    if width in updates or height in updates:
        if "landscape" in updates and after[width] != after[height] and (after[width] > after[height]) != after["landscape"]:
            raise ToolError("invalid_arguments", "The dimensions disagree with the requested orientation.")
        if "landscape" not in updates:
            after["landscape"] = after[width] > after[height]
    if (not 20 <= after[width] <= 2000 or not 20 <= after[height] <= 2000
            or any(not 0 <= after[key] <= 300 for key in PAGE_PROPERTIES if key.endswith("_margin_mm"))
            or after["left_margin_mm"] + after["right_margin_mm"] + 1 >= after[width]
            or after["top_margin_mm"] + after["bottom_margin_mm"] + 1 >= after[height]):
        raise ToolError("invalid_arguments", "Page dimensions and margins must leave room for the body.")

    changed = {key for key in PAGE_PROPERTIES if before[key] != after[key]}
    if "landscape" in changed:
        changed.update((width, height))

    def set_page(values):
        for key, prop in PAGE_PROPERTIES.items():
            if key in changed:
                setattr(style, prop, values[key] if key == "landscape" else round(values[key] * 100))

    return _restorable(lambda: set_page(after), lambda: set_page(before)), [_meta("page_setup", name, before, after)]


def _header_footer(document, elements, descriptions, details, args):
    action = args["action"]
    required = ("page_style", "region", "text") if action == "header_footer" else ("page_style", "region")
    expect(args, required)
    name, region = args["page_style"], args["region"]
    style = _page_style(document, name)
    before = _region(style, region)
    if not before["editable"]:
        raise ToolError("unsupported_content", "Use a shared header or footer without fields, objects or mixed text formatting.")
    value = args.get("text", "")
    paragraphs([value])

    prefix = region.capitalize()

    def apply():
        if not before["enabled"]:
            setattr(style, prefix + "IsOn", True)
        body = getattr(style, prefix + "Text")
        if action == "header_footer":
            body.setString(value)
        else:
            page_number = document.createInstance("com.sun.star.text.textfield.PageNumber")
            body.insertTextContent(body.createTextCursorByRange(body.getEnd()), page_number, False)

    def restore():
        if not before["enabled"]:
            setattr(style, prefix + "IsOn", False)
        else:
            getattr(style, prefix + "Text").setString(before["text"])

    return _restorable(apply, restore), [_meta(action, f"{name}/{region}", before["text"],
                                               value if action == "header_footer" else {"append_field": "page_number"})]


def _note(document, elements, descriptions, details, args):
    action, kind = args["action"], args.get("note_kind")
    if action == "insert_note":
        expect(args, ("paragraphs", "text", "note_kind"))
        index, anchor = _anchor(elements, descriptions, args)
        value = args["text"]
        paragraphs([value])

        def apply():
            note = document.createInstance("com.sun.star.text." + kind.capitalize())
            body = document.getText()
            body.insertTextContent(body.createTextCursorByRange(anchor.getEnd()), note, False)
            note.setString(value)

        return apply, [_meta(action, index, None, {"kind": kind, "text": value})]
    expect(args, ("note_kind", "note_index", "text") if action == "edit_note" else ("note_kind", "note_index"))
    index = args["note_index"]
    collection = _note_collection(document, kind)
    if not 0 <= index < collection.getCount():
        raise ToolError("invalid_target", "Use a note index from inspect notes.")
    note = collection.getByIndex(index)
    if not _one_plain(note):
        raise ToolError("unsupported_content", "Only a note containing one plain paragraph can be edited or removed.")
    value = args.get("text")
    if value is not None:
        paragraphs([value])

    def apply():
        if action == "edit_note":
            note.setString(value)
        else:
            note.getAnchor().getText().removeTextContent(note)

    return apply, [_meta(action, f"{kind}/{index}", note.getString(), value)]


def _link(document, elements, descriptions, details, args):
    expect(args, ("paragraphs", "url"))
    index = args["paragraphs"][0] if len(args["paragraphs"]) == 1 else -1
    if not 0 <= index < len(elements) or descriptions[index]["kind"] not in {"paragraph", "heading"}:
        raise ToolError("invalid_target", "Use one body paragraph or heading as the link target.")
    before = descriptions[index]
    if not before["text"] or set(before["protected_content"]) - {"Hyperlink"}:
        raise ToolError("unsupported_content", "The target must be text without protected objects.")
    url = args["url"]
    if url:
        try:
            parts = urlsplit(url)
            valid = ((parts.scheme in {"http", "https"} and parts.hostname and not parts.username)
                     or (url.startswith("#") and len(url) > 1))
        except ValueError:
            valid = False
        if not valid or len(url) > 2048 or any(ord(ch) <= 32 or ord(ch) == 127 for ch in url):
            raise ToolError("invalid_arguments", "Use an HTTP(S) URL, a #bookmark, or empty text to clear.")
    paragraph = elements[index]

    def apply():
        cursor = paragraph.getText().createTextCursorByRange(paragraph)
        cursor.HyperLinkURL = url

    return apply, [_meta("link", index, before["links"], url)]


def _tracking(document):
    return {"record_changes": bool(document.RecordChanges), "show_changes": bool(document.ShowChanges)}


def _track_changes(document, elements, descriptions, details, args):
    expect(args, ("record_changes",))
    before, after = details["tracking"]["record_changes"], args["record_changes"]
    if _review_protected(document):
        raise ToolError("unsupported_content", "Change tracking is protected in this document.")

    def apply():
        document.RecordChanges = after
        if bool(document.RecordChanges) != after:
            raise ToolError("review_failed", "Writer did not change the recording setting.")

    return _restorable(apply, lambda: setattr(document, "RecordChanges", before)), [
        _meta("track_changes", "document", {"record_changes": before}, {"record_changes": after})
    ]


def _review_protected(document):
    key = document.RedlineProtectionKey
    return bool(getattr(key, "value", key))


def _review_changes(document, elements, descriptions, details, args):
    decision = args.get("decision")
    single = decision in {"accept", "reject"}
    expect(args, ("decision", "change_index") if single else ("decision",))
    import uno

    records = [item for item in details["review"] if item["kind"] == "tracked_change"]
    if not records:
        raise ToolError("no_matches", "The document has no tracked changes to review.")
    if _review_protected(document):
        raise ToolError("unsupported_content", "Change tracking is protected in this document.")
    redlines = list(_iterate(document.getRedlines()))
    remaining = []
    if single:
        index = args["change_index"]
        if not 0 <= index < len(records):
            raise ToolError("invalid_target", "Use a tracked change index from inspect review.")
        selected = redlines[index]
        start, end = selected.RedlineStart, selected.RedlineEnd
        if start is None or end is None:
            raise ToolError("unsupported_content", "The tracked change has no selectable text range.")
        remaining = [change.RedlineIdentifier for i, change in enumerate(redlines) if i != index]
    command = {
        "accept": "AcceptTrackedChange", "reject": "RejectTrackedChange",
        "accept_all": "AcceptAllTrackedChanges", "reject_all": "RejectAllTrackedChanges",
    }[decision]
    controller = document.getCurrentController()
    address = uno.createUnoStruct("com.sun.star.util.URL")
    address.Complete, address.Protocol, address.Path = ".uno:" + command, ".uno:", command
    dispatch = controller.getFrame().queryDispatch(address, "_self", 0) if controller else None
    if dispatch is None:
        raise ToolError("unsupported_content", "This Writer session cannot review tracked changes.")

    def apply():
        if single:
            cursor = controller.getViewCursor()
            cursor.gotoRange(start, False)
            cursor.gotoRange(end, True)
        dispatch.dispatch(address, ())
        if [change.RedlineIdentifier for change in _iterate(document.getRedlines())] != remaining:
            raise ToolError("review_failed", "Writer did not apply exactly the requested review decision.")

    return apply, [_meta("review_changes", index if single else "document",
                         records[index] if single else {"tracked_changes": len(records)}, decision)]


OPERATIONS = {
    "insert_paragraphs": _insert_paragraphs, "insert_table": _insert_table,
    "set_cells": _set_cells, "table_axis": _table_axis, "page_setup": _setup,
    "header_footer": _header_footer, "page_number": _header_footer,
    "insert_note": _note, "edit_note": _note, "delete_note": _note,
    "link": _link, "track_changes": _track_changes, "review_changes": _review_changes,
}
