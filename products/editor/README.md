# Text Editor

The complete native Yu OS Text Editor now lives in this product:
`yuos-edit`, its UI, seven MCP tools, localization, resources and packaging.
Installed identity `/usr/bin/yuos-edit` / `com.yuos.Edit` is unchanged.

See [MODULE.md](MODULE.md) for ownership and validation, and
[native/PROVENANCE.md](native/PROVENANCE.md) for upstream licensing.
OS desktop packages consume an immutable source revision; no source cache,
credentials, user documents or editor settings are shipped by this move.

The interactive AI actions and MCP AI tools share the gated SDK helper as
`yuos-edit`, with strict safety and `external-content` origin. Monthly usage
counts against the owner-set App AI limit. Interactive requests operate on the unsaved
buffer; file-based MCP requests retain controlled bounded reads. Summaries,
explanations and replacement proposals do not silently save files or memory.
