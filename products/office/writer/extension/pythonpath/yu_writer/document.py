"""Writer document operations over the LibreOffice UNO document model.

These functions receive a live `com.sun.star.text.TextDocument`. They never
execute macros or arbitrary UNO calls requested by a model. Content they cannot
change faithfully is reported and refused instead of being flattened.
"""

import hashlib
import json

from . import structure
from .catalog import ToolError

MAX_BLOCKS = 20_000
MAX_PAGE = 500
MAX_RETURNED_TEXT = 64_000
MAX_EDIT_PARAGRAPHS = 200
MAX_PARAGRAPH_TEXT = 20_000
MAX_STYLES = 500
ALIGNMENTS = {"left": 0, "right": 1, "justify": 2, "center": 3}
ALIGNMENT_NAMES = {value: name for name, value in ALIGNMENTS.items()}
BOLD, NORMAL = 150.0, 100.0
FORMAT_ARGS = (
    "paragraph_style", "alignment", "bold", "italic", "underline", "font_name", "font_size",
    "numbering_style", "outline_level", "left_indent_mm", "space_after_mm", "page_break_before",
)
NOT_COVERED = (
    "table formulas and merged-cell resizing", "separate first/left/right header and footer editing",
    "text frames and shapes", "comment editing",
)
# Portion types that rewriting a paragraph's text would silently delete or detach.
PLAIN_PORTIONS = {"Text", "SoftPageBreak"}


def _slant(value):
    return getattr(value, "value", value)


def _portions(paragraph):
    enumeration = paragraph.createEnumeration()
    while enumeration.hasMoreElements():
        yield enumeration.nextElement()


def _char_format(portion):
    return {
        "bold": portion.CharWeight >= BOLD,
        "italic": _slant(portion.CharPosture) in {"ITALIC", "OBLIQUE"},
        "underline": portion.CharUnderline != 0,
        "font_name": portion.CharFontName,
        "font_size": round(float(portion.CharHeight), 2),
    }


def _portion_signature(portion):
    return [
        portion.TextPortionType, portion.getString(), float(portion.CharWeight),
        str(_slant(portion.CharPosture)), int(portion.CharUnderline), portion.CharFontName,
        round(float(portion.CharHeight), 2), int(portion.CharColor), portion.CharStyleName,
        portion.HyperLinkURL,
    ]


def _describe_paragraph(index, paragraph):
    blocked, formats, signature = set(), [], []
    for portion in _portions(paragraph):
        kind = portion.TextPortionType
        signature.append(_portion_signature(portion))
        if kind not in PLAIN_PORTIONS:
            blocked.add(kind)
        elif kind == "Text" and portion.getString():
            if portion.HyperLinkURL:
                blocked.add("Hyperlink")
            formats.append(tuple(signature[-1][2:]))
    anchored = paragraph.createContentEnumeration("com.sun.star.text.TextContent")
    if anchored.hasMoreElements():
        blocked.add("AnchoredObject")
    level = int(paragraph.OutlineLevel)
    text = paragraph.getString()
    uniform = len(set(formats)) <= 1
    first = next((p for p in _portions(paragraph) if p.TextPortionType == "Text"), paragraph)
    description = {
        "index": index,
        "kind": "heading" if level else "paragraph",
        "text": text,
        "style": paragraph.ParaStyleName,
        "heading_level": level,
        "alignment": ALIGNMENT_NAMES.get(int(paragraph.ParaAdjust), "other"),
        "list_label": paragraph.ListLabelString or "",
        "numbering_style": paragraph.NumberingStyleName or "",
        "left_indent_mm": paragraph.ParaLeftMargin / 100,
        "space_after_mm": paragraph.ParaBottomMargin / 100,
        "page_break_before": _slant(paragraph.BreakType) in {"PAGE_BEFORE", "PAGE_BOTH"},
        "format": _char_format(first) if uniform else None,
        "mixed_formatting": not uniform,
        "rewritable": not blocked,
        "protected_content": sorted(blocked),
        "links": sorted({row[-1] for row in signature if row[-1]}),
    }
    fingerprint = [description, signature, str(_slant(paragraph.BreakType))]
    return description, fingerprint


def _blocks(document):
    enumeration = document.getText().createEnumeration()
    index = 0
    table_cells = structure.MAX_CELLS
    while enumeration.hasMoreElements():
        if index >= MAX_BLOCKS:
            raise ToolError("document_too_large", f"The document has more than {MAX_BLOCKS} body blocks.")
        element = enumeration.nextElement()
        if element.supportsService("com.sun.star.text.Paragraph"):
            description, fingerprint = _describe_paragraph(index, element)
        elif element.supportsService("com.sun.star.text.TextTable"):
            description = {
                "index": index, "kind": "table", "name": element.getName(),
                "rows": element.getRows().getCount(), "columns": element.getColumns().getCount(),
                "rewritable": False, "protected_content": ["Table"],
            }
            cells = structure.table_fingerprint(element, table_cells)
            table_cells -= len(cells)
            fingerprint = [description, cells]
        else:
            description = {"index": index, "kind": "other", "rewritable": False,
                           "protected_content": ["Unsupported"]}
            fingerprint = ["other"]
        yield element, description, fingerprint
        index += 1


def _count(supplier, method):
    getter = getattr(supplier, method, None)
    return getter().getCount() if getter is not None else 0


def _objects(document):
    redlines = 0
    if hasattr(document, "getRedlines"):
        enumeration = document.getRedlines().createEnumeration()
        while enumeration.hasMoreElements() and redlines < 10_000:
            enumeration.nextElement()
            redlines += 1
    return {
        "tables": _count(document, "getTextTables"),
        "images": _count(document, "getGraphicObjects"),
        "frames": _count(document, "getTextFrames"),
        "embedded_objects": _count(document, "getEmbeddedObjects"),
        "sections": _count(document, "getTextSections"),
        "footnotes": _count(document, "getFootnotes"),
        "endnotes": _count(document, "getEndnotes"),
        "tracked_changes": redlines,
    }


def paragraph_styles(document):
    family = document.getStyleFamilies().getByName("ParagraphStyles")
    styles = []
    for name in family.getElementNames()[:MAX_STYLES]:
        style = family.getByName(name)
        styles.append({"name": name, "display_name": style.DisplayName, "in_use": bool(style.isInUse())})
    return styles


def _snapshot(document):
    elements, descriptions, fingerprints = [], [], []
    for element, description, fingerprint in _blocks(document):
        elements.append(element)
        descriptions.append(description)
        fingerprints.append(fingerprint)
    details = structure.capture(document)
    canonical = json.dumps([fingerprints, details], ensure_ascii=False, separators=(",", ":"))
    return elements, descriptions, hashlib.sha256(canonical.encode("utf-8")).hexdigest(), details


def snapshot(document):
    """Return body elements, descriptions and the revision of the editable document."""
    elements, descriptions, current, _ = _snapshot(document)
    return elements, descriptions, current


def revision(document):
    return snapshot(document)[2]


def inspect(document, start, limit, view="body", table_name=None):
    if start < 0 or not 1 <= limit <= MAX_PAGE:
        raise ToolError("invalid_arguments", f"Use start >= 0 and 1 <= limit <= {MAX_PAGE}.")
    _, descriptions, current, details = _snapshot(document)
    records = structure.view_records(document, details, descriptions, view, table_name)
    page, budget, truncated = [], MAX_RETURNED_TEXT, False
    for record in records[start:start + limit]:
        fields = [(record, key) for key in ("text", "comment")]
        fields += [(record[area], "text") for area in ("header", "footer") if area in record]
        size = sum(len(item.get(key, "")) for item, key in fields)
        if size <= budget:
            page.append(record)
            budget -= size
            continue
        truncated = True
        if not page:
            # Return a bounded record and a progressing cursor, even when one item
            # consumes the whole page budget.
            limited = {key: value.copy() if isinstance(value, dict) else value for key, value in record.items()}
            fields = [(limited, key) for key in ("text", "comment")]
            fields += [(limited[area], "text") for area in ("header", "footer") if area in limited]
            for item, key in fields:
                if key in item:
                    item[key] = item[key][:budget]
                    budget -= len(item[key])
            page.append({**limited, "text_truncated": True})
        break
    following = start + len(page)
    return {
        "revision": current,
        "view": view,
        "block_count": len(records),
        "start": start,
        "blocks": page,
        "next_start": following if following < len(records) else None,
        "truncated": truncated,
        "paragraph_styles": paragraph_styles(document),
        "numbering_styles": structure.list_styles(document),
        "tracking": details["tracking"],
        "objects": _objects(document),
        "not_covered": list(NOT_COVERED),
    }


def _validated_targets(descriptions, indices):
    if not indices:
        raise ToolError("invalid_arguments", "Select at least one paragraph.")
    if len(indices) > MAX_EDIT_PARAGRAPHS:
        raise ToolError("invalid_arguments", f"Edit at most {MAX_EDIT_PARAGRAPHS} paragraphs per call.")
    if len(set(indices)) != len(indices):
        raise ToolError("invalid_arguments", "Each paragraph may appear only once.")
    for index in indices:
        if not 0 <= index < len(descriptions):
            raise ToolError("invalid_target", f"Block {index} does not exist.")
        if descriptions[index]["kind"] not in {"paragraph", "heading"}:
            raise ToolError("unsupported_content", f"Block {index} is a {descriptions[index]['kind']}, not a paragraph.")


def plan(document, args):
    """Validate a preview/apply request against the current document model."""
    elements, descriptions, current, details = _snapshot(document)
    if args["revision"] != current:
        raise ToolError("stale_revision", "The document changed; inspect it again before editing.")
    action = args["action"]
    if action in structure.OPERATIONS:
        edit, changes = structure.OPERATIONS[action](document, elements, descriptions, details, args)
        return edit, current, changes, []

    if action == "format":
        structure.expect(args, ("paragraphs",), FORMAT_ARGS)
    elif action == "rewrite":
        structure.expect(args, ("paragraphs", "texts"), ("flatten_formatting",))
    else:
        structure.expect(args, ("paragraphs", "find", "replacement"), ("flatten_formatting",))
    indices = args["paragraphs"]
    _validated_targets(descriptions, indices)
    changes, warnings = [], []
    formatting = {key: args[key] for key in FORMAT_ARGS if key in args}
    if action in {"rewrite", "find_replace"}:
        if action == "rewrite" and len(args["texts"]) != len(indices):
            raise ToolError("invalid_arguments", "Provide one replacement text per paragraph.")
        if action == "find_replace" and (not args["find"] or args["find"] == args["replacement"]):
            raise ToolError("invalid_arguments", "Provide nonempty find text and a different replacement.")
        if action == "find_replace":
            structure.paragraphs([args["find"], args["replacement"]])
        updates = []
        flatten = args.get("flatten_formatting", False)
        for position, index in enumerate(indices):
            before = descriptions[index]
            if action == "rewrite":
                text = args["texts"][position]
            else:
                if args["find"] not in before["text"]:
                    continue
                size = len(before["text"]) + before["text"].count(args["find"]) * (len(args["replacement"]) - len(args["find"]))
                if size > MAX_PARAGRAPH_TEXT:
                    raise ToolError("invalid_arguments", f"Replacement for block {index} would be too long.")
                text = before["text"].replace(args["find"], args["replacement"])
            if not before["rewritable"]:
                raise ToolError("unsupported_content",
                                f"Block {index} contains {', '.join(before['protected_content'])}; rewriting would remove it.")
            if before["mixed_formatting"]:
                if not flatten:
                    raise ToolError("mixed_formatting",
                                    f"Block {index} has mixed character formatting; set flatten_formatting to accept losing it.")
                warnings.append(f"Block {index} will take the character formatting of its first text run.")
            changes.append({"paragraph": index, "before": before["text"], "after": text})
            updates.append((elements[index], text))
        structure.paragraphs([text for _, text in updates])
        if not updates:
            raise ToolError("no_matches", "None of the selected paragraphs contains the requested text.")

        def edit():
            for paragraph, text in updates:
                paragraph.setString(text)
    else:
        if not formatting:
            raise ToolError("invalid_arguments", "Provide at least one formatting argument.")
        if "paragraph_style" in formatting:
            names = document.getStyleFamilies().getByName("ParagraphStyles")
            if not names.hasByName(formatting["paragraph_style"]):
                raise ToolError("invalid_arguments", "Unknown paragraph style; use a name returned by inspect.")
        if "font_name" in formatting and not 0 < len(formatting["font_name"]) <= 128:
            raise ToolError("invalid_arguments", "font_name must contain 1 to 128 characters.")
        if "font_size" in formatting and not 1 <= formatting["font_size"] <= 999:
            raise ToolError("invalid_arguments", "font_size must be between 1 and 999 points.")
        if "numbering_style" in formatting and formatting["numbering_style"] not in structure.list_styles(document):
            raise ToolError("invalid_arguments", "Unknown numbering style; use a name returned by inspect.")
        if "outline_level" in formatting and not 0 <= formatting["outline_level"] <= 10:
            raise ToolError("invalid_arguments", "outline_level must be from 0 to 10.")
        for key in ("left_indent_mm", "space_after_mm"):
            if key in formatting and not 0 <= formatting[key] <= 300:
                raise ToolError("invalid_arguments", f"{key} must be between 0 and 300 mm.")
        for index in indices:
            before = descriptions[index]
            if "page_break_before" in formatting and _slant(elements[index].BreakType) not in {"NONE", "PAGE_BEFORE"}:
                raise ToolError("unsupported_content", "The paragraph already has a different kind of break.")
            changes.append({
                "paragraph": index, "text": before["text"],
                "before": {"style": before["style"], "alignment": before["alignment"],
                           "format": before["format"], "numbering_style": before["numbering_style"],
                           "outline_level": before["heading_level"], "left_indent_mm": before["left_indent_mm"],
                           "space_after_mm": before["space_after_mm"], "page_break_before": before["page_break_before"]},
                "after": formatting,
            })

        def edit():
            for index in indices:
                _format(elements[index], args)
    return edit, current, changes, warnings


def _format(paragraph, args):
    import uno

    if "paragraph_style" in args:
        paragraph.ParaStyleName = args["paragraph_style"]
    if "alignment" in args:
        paragraph.ParaAdjust = ALIGNMENTS[args["alignment"]]
    if "numbering_style" in args:
        paragraph.NumberingStyleName = "" if args["numbering_style"] == "No List" else args["numbering_style"]
    if "outline_level" in args:
        paragraph.OutlineLevel = args["outline_level"]
    if "left_indent_mm" in args:
        paragraph.ParaLeftMargin = round(args["left_indent_mm"] * 100)
    if "space_after_mm" in args:
        paragraph.ParaBottomMargin = round(args["space_after_mm"] * 100)
    if "page_break_before" in args:
        paragraph.BreakType = uno.Enum("com.sun.star.style.BreakType", "PAGE_BEFORE" if args["page_break_before"] else "NONE")
    cursor = paragraph.getText().createTextCursorByRange(paragraph)
    if "bold" in args:
        cursor.CharWeight = BOLD if args["bold"] else NORMAL
    if "italic" in args:
        cursor.CharPosture = uno.Enum("com.sun.star.awt.FontSlant", "ITALIC" if args["italic"] else "NONE")
    if "underline" in args:
        cursor.CharUnderline = 1 if args["underline"] else 0
    if "font_name" in args:
        cursor.CharFontName = args["font_name"]
    if "font_size" in args:
        cursor.CharHeight = float(args["font_size"])


def apply_changes(edit):
    edit()
