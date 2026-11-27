"""yuos_sdk — public AI/agent SDK for Yu OS, Python edition.

This package is the AI-facing surface a third-party Linux developer
uses when their app touches the kernel's AI features. Everything in
here mirrors a member of the wire protocol at ``yuos-sdk/wire/v1/``
and is generated or hand-written to satisfy that contract.

Public modules
--------------
- :mod:`yuos_sdk.app`      — mark the ``@command`` and ``@desktop``
  functions Yu OS starts for the App's command line and desktop window.
- :mod:`yuos_sdk.ai`       — the user's configured models.
- :mod:`yuos_sdk.tools`    — call fixed kernel catalog tools
  (``tools.call``, ``tools.catalog``, ``tools.for_chat``), not other Apps.
- :mod:`yuos_sdk.gui`      — desktop GUI bootstrap: the kernel context
  handed to an app launched via ``yu app <id> --gui`` (toolkit-agnostic).
- :mod:`yuos_sdk.mcp`      — manifest-bound MCP App service runtime.
- :mod:`yuos_sdk.kernel`   — cancellable installed-CLI primitive transport.
- :mod:`yuos_sdk.objects`  — pure App object identifiers, without discovery,
  filesystem access, or dispatch.
- :mod:`yuos_sdk.yuos_session` — reference implementation for
  third-party agents that want to attach to a live ``yuos`` session.
- :mod:`yuos_sdk.generated` — typed wire bindings emitted from the
  wire-v1 JSON Schemas. Do not hand-edit; re-run
  ``python3 yuos-sdk/wire/codegen.py`` instead.

Typical usage::

    from yuos_sdk.ai import chat
    from yuos_sdk.app import command

    @command("summarize")
    def summarize(args):
        return {"summary": chat(args[0]).output.text}

Capability gating (``policy.require``) and the COW snapshot helper
live in the **internal** ``yuos_runtime`` package — they are
implementation details of the yuos bundled apps, not part of the
public SDK surface.
"""

from . import (
    ai,
    app,
    app_calls,
    browser_bridge,
    channels,
    egress,
    events,
    file_changes,
    files,
    gui,
    kernel,
    manifest,
    mcp,
    memory,
    network_diagnostics,
    objects,
    policy,
    smtp,
    snapshot,
    tools,
)

__all__ = [
    "ai",
    "app",
    "app_calls",
    "browser_bridge",
    "channels",
    "egress",
    "events",
    "file_changes",
    "files",
    "gui",
    "kernel",
    "manifest",
    "mcp",
    "memory",
    "network_diagnostics",
    "objects",
    "policy",
    "smtp",
    "snapshot",
    "tools",
]
__version__ = "0.1.0.dev2026112401"
