"""Writer document operations over the LibreOffice UNO document model.

These functions receive a live `com.sun.star.text.TextDocument`. They never
execute macros or arbitrary UNO calls requested by a model. Content they cannot
change faithfully is reported and refused instead of being flattened.
"""

import hashlib
import json

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
FORMAT_ARGS = ("paragraph_style", "alignment", "bold", "italic", "underline", "font_name", "font_size")
NOT_COVERED = (
    "table cell text", "headers and footers", "text frames and shapes",
    "footnote and endnote text", "comments", "tracked-change review",
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
        "format": _char_format(first) if uniform else None,
        "mixed_formatting": not uniform,
        "rewritable": not blocked,
        "protected_content": sorted(blocked),
    }
    fingerprint = [description["style"], level, description["alignment"], description["list_label"], signature]
    return description, fingerprint


def _blocks(document):
    enumeration = document.getText().createEnumeration()
    index = 0
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
            cells = [element.getCellByName(name).getString() for name in element.getCellNames()]
            fingerprint = ["table", description["name"], cells]
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


def snapshot(document):
    """Return elements, descriptions and the model revision of the whole body."""
    elements, descriptions, fingerprints = [], [], []
    for element, description, fingerprint in _blocks(document):
        elements.append(element)
        descriptions.append(description)
        fingerprints.append(fingerprint)
    canonical = json.dumps(fingerprints, ensure_ascii=False, separators=(",", ":"))
    return elements, descriptions, hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def revision(document):
    return snapshot(document)[2]


def inspect(document, start, limit):
    if start < 0 or not 1 <= limit <= MAX_PAGE:
        raise ToolError("invalid_arguments", f"Use start >= 0 and 1 <= limit <= {MAX_PAGE}.")
    _, descriptions, current = snapshot(document)
    page, budget, truncated = [], MAX_RETURNED_TEXT, False
    for description in descriptions[start:start + limit]:
        text = description.get("text")
        if text is not None and len(text) > budget:
            truncated = True
            break
        budget -= len(text or "")
        page.append(description)
    following = start + len(page)
    return {
        "revision": current,
        "block_count": len(descriptions),
        "start": start,
        "blocks": page,
        "next_start": following if following < len(descriptions) else None,
        "truncated": truncated,
        "paragraph_styles": paragraph_styles(document),
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
    elements, descriptions, current = snapshot(document)
    if args["revision"] != current:
        raise ToolError("stale_revision", "The document changed; inspect it again before editing.")
    indices = args["paragraphs"]
    _validated_targets(descriptions, indices)
    action, changes, warnings = args["action"], [], []
    formatting = {key: args[key] for key in FORMAT_ARGS if key in args}
    if action == "rewrite":
        texts = args.get("texts")
        if formatting:
            raise ToolError("invalid_arguments", "Rewrite does not accept formatting arguments.")
        if texts is None or len(texts) != len(indices):
            raise ToolError("invalid_arguments", "Provide one replacement text per paragraph.")
        flatten = args.get("flatten_formatting", False)
        for index, text in zip(indices, texts):
            before = descriptions[index]
            if len(text) > MAX_PARAGRAPH_TEXT or any(ch in text for ch in "\r\n\x0b\x0c\u2028\u2029"):
                raise ToolError("invalid_arguments",
                                f"Replacement for block {index} must be one paragraph of at most {MAX_PARAGRAPH_TEXT} characters.")
            if not before["rewritable"]:
                raise ToolError("unsupported_content",
                                f"Block {index} contains {', '.join(before['protected_content'])}; rewriting would remove it.")
            if before["mixed_formatting"]:
                if not flatten:
                    raise ToolError("mixed_formatting",
                                    f"Block {index} has mixed character formatting; set flatten_formatting to accept losing it.")
                warnings.append(f"Block {index} will take the character formatting of its first text run.")
            changes.append({"paragraph": index, "before": before["text"], "after": text})
    else:
        if "texts" in args or "flatten_formatting" in args:
            raise ToolError("invalid_arguments", "Format does not accept texts or flatten_formatting.")
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
        for index in indices:
            before = descriptions[index]
            changes.append({
                "paragraph": index, "text": before["text"],
                "before": {"style": before["style"], "alignment": before["alignment"],
                           "format": before["format"]},
                "after": formatting,
            })
    return elements, current, changes, warnings


def apply_changes(document, elements, args):
    import uno

    for position, index in enumerate(args["paragraphs"]):
        paragraph = elements[index]
        if args["action"] == "rewrite":
            paragraph.setString(args["texts"][position])
            continue
        if "paragraph_style" in args:
            paragraph.ParaStyleName = args["paragraph_style"]
        if "alignment" in args:
            paragraph.ParaAdjust = ALIGNMENTS[args["alignment"]]
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
