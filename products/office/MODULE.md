# Yu Office

Yu Office is a downstream LibreOffice desktop product with its own `yu-office`
App identity, launcher and profile. It does not own the OS launcher or Agent,
and does not replace the system `libreoffice`/`soffice` commands, profiles or
file associations. The Yu OS Apps catalog delivers `yuos-app-office-desktop`;
the distribution's LibreOffice remains a separate Linux Apps entry.

| Path | Responsibility |
| --- | --- |
| `upstream.json` | Exact Document Foundation source archive URL, version, byte length and SHA-256 |
| `patches/series`, `patches/*.patch` | Complete ordered downstream changes; initially the Yu Office product name |
| `source.py` | Verify the cached/downloaded archive, extract a fresh tree and apply every listed patch |
| `native/` | Configure and compile the patched source, smoke-check its identity, and stage the private runtime and Host launcher |
| `apps/yu-office/app.json`, `apps/yu-office/server.py` | Independent GUI and SDK MCP entries under the same signed App identity |
| `writer/extension/` | Bundled Writer AI extension (`org.yuos.office.writer-ai`) and the shared `yu_writer` catalog, UNO operations and dispatcher used by both AI surfaces |

## Writer tools (shared by the in-Office assistant and the System Agent)

`yu-office.writer.inspect`, `yu-office.writer.preview` and
`yu-office.writer.apply` are declared once in the signed `app.json`. That
declaration is the only catalog: `yu_writer.catalog` reads it for names,
argument schemas, choices and validation, and `yu_writer.tools.call` is the
only dispatcher. The MCP server binds each manifest tool to that dispatcher;
the Writer menu **Yu AI → Writer Assistant…** calls the same dispatcher on the
live document. Tests check the derived schema equals the SDK's MCP schema and
that handlers and catalog name the same tools.

Operations use LibreOffice's UNO Writer model, not XML editing. `inspect`
returns top-level body paragraphs (text, style, outline level, alignment,
uniform character attributes) and a revision. `preview` and `apply` take that
revision, an `action` (`rewrite` or `format`) and paragraph indices. Rewrites
replace whole paragraph text. Mixed character formatting is refused unless
`flatten_formatting` is true; fields, frames, images and other non-text
content in a target paragraph always refuse. Format applies the
requested paragraph style, alignment, bold/italic/underline, font name or size.
Tables, headers/footers, frames, footnotes, comments and tracked changes are
reported but not editable. A stale revision, unknown paragraph, invalid
argument or unsupported content fails without changes. Macros and arbitrary
UNO calls are never exposed.

Target binding comes from the adapter, never from model arguments:

- **In-Office assistant**: the current live Writer document, including unsaved
  edits. `path` is rejected. Apply is one undo step titled “Yu Writer AI” and
  does not save. AI is optional: the assistant calls
  `yuos_sdk.ai.chat(..., origin="external-content", system=...)` only on an
  explicit button press, without SDK `tools=`. The model must return a closed
  `{tool, arguments}` preview proposal, which is validated locally against the
  catalog and the user's selection, previewed, and applied only after the user
  confirms. Manual formatting needs no AI. Missing SDK, grant, budget or safety
  refusal is shown as an error and leaves the document unchanged. The App
  declares the `ai` policy and the optional install-time need
  `ai.chat.untrusted` scoped to `$configured-ai-model`; there is no App-owned
  enablement flow or fallback.
- **System Agent (MCP)**: an exact OS-authorized `.docx`/`.odt` path. Wildcard,
  missing, non-regular, oversized or signed (on edit) targets fail. A private
  headless soffice (private profile, no macros, no link updates) works on a
  scratch copy. The current `PathDocumentSource` saves by revision check,
  snapshot and atomic replace of the path. This interim adapter is **not** a
  managed save: a task App runs as a separate UID with exact bind mounts, so
  owner-private files may be unreadable and replacing a bound original is not
  supported. It is replaceable by an OS staged read/commit capability without
  changing the tools (`system.file.replace` is limited to 64 KiB today).

DOCX round trips go through LibreOffice's filters and may normalize content.

## Legacy agent tools

The desktop package also serves `yu-office.read` and `yu-office.replace_text`
through the public Python SDK. The same signed launcher selects MCP or the
unchanged GUI; the GUI does not use or require an active Agent.

The tools read the main body of `.docx` and `.odt` Writer documents. Edits
replace one exact, case-sensitive match within a single formatted text run,
leaving other package members unchanged. They do not execute macros or
follow external links. Spreadsheets, presentations and other formats are
not part of this tool set. Reads return up to 64,000 characters and the file's
SHA-256 revision. Pass that revision as `expected_sha256` to edit; a stale
revision, an ambiguous match or a match across formatting runs fails without
saving. Edits reject signed documents.

Each call requires OS authorization for the exact path; edits also require
write permission and use the OS snapshot service before atomically saving.
Neither installation nor the App's GUI grants Agent file access.

## Patch and build

From the repository root on a Linux filesystem:

```sh
python3 products/office/source.py check
python3 products/office/source.py prepare --output build/office-work
```

Develop upstream-relative patches in the prepared checkout and commit them
under `patches/` with names such as `0002-writer-toolbar.patch`. List each one
in `patches/series` in application order. Do not edit the archive cache or
commit the extracted tree. The release needs a new independent App package
version when its bytes change.

The release uses the existing Debian 13 App build dependencies and
`release-apps.yml` with `select: office`. Use `build_only: true` to validate
both architectures without publishing or using signing material. For an
unsigned amd64 package build with those dependencies already installed:

```sh
python3 tools/release.py plan --select office --version 0.1.0
python3 tools/release.py build --plan build/release-plan.json --architecture amd64
```

The runtime is built from a fresh verified source tree every time. A missing,
unlisted or failed patch stops the build before packaging. There is no
installed LibreOffice binary, upstream binary download or unpatched fallback.
Publication requires the existing publisher, package review and signed APT
workflow. It is not an automatic consequence of a source push.

## Provenance

The upstream source is [LibreOffice](https://www.libreoffice.org/), principally
MPL 2.0 with the other licenses identified by upstream. The package retains
upstream `COPYING` and the generated runtime license/notice files. The exact
upstream archive, the patches, and build instructions at each release's App
source commit provide the corresponding modified source. Retain that source
and upstream build dependency sources for distributed builds. The display name
and icon are Yu Office's own; “based on LibreOffice” identifies the source and
does not imply endorsement by The Document Foundation.
