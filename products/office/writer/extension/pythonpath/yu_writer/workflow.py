"""Rehearse and replay ordered Writer actions through the existing planner."""

import hashlib
import json

from . import document as model
from .catalog import ToolError

MAX_STEPS = 8
MAX_PLAN_BYTES = 192_000
MAX_PREVIEW_BYTES = 250_000


def _encoded(value):
    return json.dumps(value, ensure_ascii=True, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("ascii")


def unique_fields(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ToolError("invalid_workflow", "A workflow step has duplicate argument names.")
        result[key] = value
    return result


def validate_steps(catalog, preview_tool, steps, revision, selected=None):
    if not isinstance(steps, list) or not 1 <= len(steps) <= MAX_STEPS:
        raise ToolError("invalid_workflow", f"Provide one to {MAX_STEPS} ordered Writer steps.")
    try:
        size = len(_encoded(steps))
    except (ValueError, RecursionError, OverflowError):
        raise ToolError("invalid_workflow", "The Writer workflow must use finite, bounded JSON values.") from None
    if size > MAX_PLAN_BYTES:
        raise ToolError("invalid_workflow", "The Writer workflow is too large.")
    result = []
    for number, step in enumerate(steps, 1):
        if not isinstance(step, dict) or {"path", "revision"} & step.keys():
            raise ToolError("invalid_workflow", f"Step {number} must be an action object without path or revision.")
        try:
            action = catalog.validate(preview_tool, {**step, "revision": revision}, bound_target=True)
        except ToolError as error:
            raise ToolError(error.code, f"Step {number}: {error.args[0]}") from None
        except OverflowError:
            raise ToolError("invalid_workflow", f"Step {number}: a numeric argument is too large.") from None
        if selected is not None:
            if not action.get("paragraphs") or not set(action["paragraphs"]) <= selected:
                raise ToolError("invalid_proposal", "The AI proposed changes outside your selection.")
            if action["action"] in {"insert_paragraphs", "insert_table"} and number != len(steps):
                raise ToolError("invalid_proposal", "Insert new body blocks last when editing a paragraph selection.")
        del action["revision"]
        result.append(action)
    return result


def parse_steps(catalog, preview_tool, raw, revision):
    if len(raw.encode("utf-8")) > MAX_PLAN_BYTES:
        raise ToolError("invalid_workflow", "The Writer workflow is too large.")
    try:
        steps = json.loads(raw, object_pairs_hook=unique_fields)
        return validate_steps(catalog, preview_tool, steps, revision)
    except (ValueError, RecursionError, OverflowError) as error:
        if isinstance(error, ToolError):
            raise
        raise ToolError("invalid_workflow", "Use a bounded JSON array of Writer action objects.") from None


def _run(target, document, steps, revision, expected=None, restores=None):
    results, remaining = [], MAX_PREVIEW_BYTES
    for index, step in enumerate(steps, 1):
        target.check_cancelled()
        try:
            edit, _, changes, warnings = model.plan(document, {**step, "revision": revision})
            item = {"step": index, "action": step["action"], "changes": changes, "warnings": warnings}
            remaining -= len(_encoded(item))
            if remaining < 0:
                raise ToolError("preview_too_large", "Split this workflow into smaller previews.")
            if expected is not None and item != expected[index - 1]:
                raise ToolError("preview_changed", "Live changes differ from the preview; preview again.")
            restore = getattr(edit, "restore", None)
            if target.bound and restore is not None and len(steps) > 1:
                raise ToolError("workflow_requires_split",
                                "In an open document, preview page settings and change-recording settings separately.")
            if restores is not None and restore is not None:
                restores.append(restore)
            model.apply_changes(edit)
            revision = model.revision(document)
        except ToolError as error:
            raise ToolError(error.code, f"Step {index}: {error.args[0]}") from None
        results.append(item)
    target.check_cancelled()
    return results, revision


def _digest(target, args, steps, preview, result_revision):
    return hashlib.sha256(_encoded([target.workflow_identity, args.get("path"), args["revision"],
                                    steps, preview, result_revision])).hexdigest()


def run(catalog, preview_tool, target, args, *, apply):
    steps = parse_steps(catalog, preview_tool, args["steps_json"], args["revision"])
    document = target.document()
    current = model.revision(document)
    if args["revision"] != current:
        raise ToolError("stale_revision", "The document changed; inspect it again before editing.")

    with target.rehearse(document) as copy:
        if model.revision(copy) != current:
            raise ToolError("workflow_unavailable", "Writer could not reproduce this document in a private preview.")
        preview, after = _run(target, copy, steps, current)
    digest = _digest(target, args, steps, preview, after)
    result = {"revision": current, "steps": preview, "preview_digest": digest, "applied": False}
    if not apply:
        return result
    if args["preview_digest"] != digest or (target.bound and model.revision(document) != current):
        raise ToolError("preview_changed", "The plan or its effects changed; preview the workflow again.")

    if target.bound:
        modified = document.isModified()
        restores = []
        try:
            with target.edit(document):
                _, actual = _run(target, document, steps, current, expected=preview, restores=restores)
                if actual != after:
                    raise ToolError("preview_changed", "Live changes differ from the preview; preview again.")
        except Exception:
            target.restore_workflow(document, restores, current, modified)
            raise
    target.check_cancelled()
    result.update(target.commit(document))
    if target.bound and restores:
        result["undo"] = None
    result.update(revision=after, previous_revision=current, applied=True)
    return result
