# Yu OS Store Third-Party Submissions

This directory holds incoming third-party `.deb` packages submitted for owner
review before entering the official Yu OS App Store (`xiaoyu-work/yuos-app`).

**Nothing submitted by a third party is ever auto-accepted or auto-published.**
Every submission is reviewed by the Store owner first.

## 1. For External Developers: Build, Pack, and Submit

1. Scaffold, lint, and test your App locally using the Yu OS CLI:
   ```bash
   yu app init my-app
   yu app lint ./my-app
   yu app test ./my-app
   ```
2. Pack your App directory into a `.deb` archive:
   ```bash
   yu app pack ./my-app --out submissions/incoming/
   ```
   This produces `submissions/incoming/yuos-app-<id>_<version>_<arch>.deb` (or
   `<id>_<version>_<arch>.deb`) containing:
   - `/usr/lib/yuos/apps/<id>/app.json` and your App entrypoints/assets
   - `DEBIAN/control` (`Package: yuos-app-<id>`, `Depends: yuos-app-runtime-v1 (>= 0.1.0)`, `X-YuOS-App-Ids: <id>`)
   - `DEBIAN/md5sums`
   - No maintainer scripts (`preinst`, `postinst`, `prerm`, `postrm`).
3. Open a Pull Request to `xiaoyu-work/yuos-app` placing the `.deb` file under
   `submissions/incoming/`. Never modify the live APT pool or Store index directly.

## 2. For the Store Owner: Review and Approve

1. Run the static security and permission review on the submitted `.deb` (never
   executes any code inside the package):
   ```bash
   python3 tools/store_submission.py review submissions/incoming/<file>.deb
   # Or machine-readable JSON:
   python3 tools/store_submission.py review submissions/incoming/<file>.deb --json
   ```
   `review` verifies:
   - Archive safety via `tools/release_payload.py` (`validate_deb`): rejects
     maintainer scripts, setuid/setgid bits, xattrs/ACLs, escaping symlinks,
     and any files outside `usr/lib/yuos/apps/<id>/` (and optional
     `usr/share/applications/`).
   - `DEBIAN/control` metadata (`Package`, `Version`, `Architecture`,
     `Depends: yuos-app-runtime-v1 (>= 0.1.0)`, `X-YuOS-App-Ids`).
   - `app.json` schema and static lint (declared entrypoints exist as regular
     files, no direct model-provider SDK imports when `"ai"` is declared).
   - Categorized permission report separating **App permissions**
     (privacy/device/AI) from **system permissions** (prominently flagged).

2. After approving the code and requested permissions, sign the App payload with
   the Store publisher key and repack a clean, deterministic `.deb`:
   ```bash
   python3 tools/store_submission.py approve submissions/incoming/<file>.deb \
     --out build/approved/yuos-app-<id>_<version>_<arch>.deb \
     --provenance-key /path/to/store-publisher-key.json
   ```
   `approve` re-runs all `review` checks, signs `/usr/lib/yuos/apps/<id>` with
   the publisher `yuos.provenance/v1` Ed25519 key, updates `DEBIAN/md5sums`, and
   outputs a verified `.deb` ready for the Store APT release pipeline.
