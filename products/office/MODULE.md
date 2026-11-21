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
| `apps/yu-office/app.json` | GUI-only App contract; no Agent dependency or MCP authority |

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
