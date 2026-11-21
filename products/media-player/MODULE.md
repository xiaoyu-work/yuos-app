# Media Player Product

| Path | Responsibility |
| --- | --- |
| `apps/yuos-player/app.json` | Seven tools and separate exact observation/control grants |
| `native/yuos-player/src/main.rs` | Original GStreamer/libcosmic UI and authoritative playback state |
| `native/yuos-player/src/argparse.rs`, `native/yuos-player/test/unit/argparse.rs` | Native media/thumbnail arguments, bounded Host GUI selector adaptation and parser regressions |
| `native/yuos-player/src/playback.rs` | Shared native state snapshots and typed UI commands |
| `native/yuos-player/src/managed.rs` | Published generic SDK exchange for the exact managed GUI; commands and status go through the original UI event loop |
| `native/yuos-player/src/access.rs` | Independent protected file reads and atomic thumbnail write/create selections through the published GUI SDK |
| `native/yuos-player/src/mpris_backend.rs` | Original MPRIS implementation, corrected name/Stop/status and live UI publication |
| `native/yuos-player/src/mpris.rs` | Native subscription connecting the MPRIS backend to the existing UI event loop |
| `native/yuos-player/src/mcp.rs` | Authenticated MCP, strict empty arguments, deadlines/cancellation and explicit installed OS client |
| `native/yuos-player/test/unit/` | Actual backend commands, state/metadata and UI message mapping |
| `native/yuos-player/test/support/mpris.rs` | Fixture-only private bus endpoints and synthetic native state |
| `native/test_process.py` | Installed binary/resources, isolated stdio MCP and fixture broker/native-MPRIS acceptance |
| `package.json` | Descriptor, native inputs, fixture executable and process-test declarations |

The native binary remains a standalone workspace with its original
`iced_video_player` / libcosmic renderer and optional feature graph.
Video uses the original WGPU shader; Tiny Skia cannot paint that primitive.
Release dependencies include the Vulkan loader and Mesa software implementation,
so software rendering does not require a GPU-device grant.
The OS owns authentication, per-owner grants, executable identity, session
selection, dispatch gating, deadlines and audit. Product code imports only
the versioned SDK, not OS providers or other Apps. Native state is projected
from the UI, never a second MCP-only cache.

The OS playback service accepts other independently authorized clients without
requiring the caller to be `yuos-player`. The native product keeps its own
identity and manifest; its exact `desktop.media.observe:yuos-player` and
`desktop.media.control:yuos-player` scopes name the fixed playback target.
The provider selects the actual running signed GUI instance and applies each
calling App's own live grants and denial policy before releasing a command and
returning a reply. Status is sampled by that same native UI, not a worker-owned
cache or mutable owner-bus name. No fixed executable path is trusted as the
package identity, and managed failures never fall back to raw D-Bus. Multiple
running target windows remain ambiguous, not an arbitrary selection.

The GUI separately requests Home metadata browsing. Optional content read and
thumbnail write declarations do not grant startup file authority. Each media
read and thumbnail write/create uses the existing protected native selection;
thumbnail output is committed through Root's same-parent scratch reservation.
Playback permission and returned filenames never grant filesystem access.
The managed Open action uses the generic OS FileChooser presentation and its
correlated metadata-only result, then the same protected file read. It never
falls back to the original owner-bus dialog when managed mediation fails.

Playback audio separately declares `device.audio:output`. The original
GStreamer pipeline sends fixed stereo PCM through the fixed OS CLI transport;
Root owns its output-only sound-server helper and lifetime. The App gets no
PipeWire, microphone or session-bus descriptor. Its native video guard retains
that transport until the pipeline closes. Silent thumbnail generation disables
audio rather than acquiring playback or capture authority.

Run from the repository root on Linux/WSL:

```sh
python3 tools/test.py media-player
python3 tools/native_build.py media-player test
python3 tools/native_build.py media-player build
python3 products/media-player/native/test_process.py
```

The process fixture requires the existing `just`, `bubblewrap` and
`dbus-daemon` tools. It accepts `--binary <binary> --source <native-inputs>`
for an immutable OS-composed build, and `--yuos-binary <yu>` to exercise the
actual installed-path CLI against its private broker fixture. The OS repository
owns provider/authority and authenticated dispatch tests. Fixtures never
open user files, start playback devices, or connect to a live desktop bus.

`native_payload` selects the real `bin/yuos-player` for GUI/MCP and preserves
the original resources, thumbnailer metadata and licenses. The primary legacy
command enters the common Host; it does not bypass admission for thumbnailing.
After the existing MCP dispatch, the App-owned `yuos-app-gui-argv` library removes
one exact argv[1] `--gui` only in SDK GUI mode. The existing lexer still owns
option values, URL order, local-path canonicalization, warning-and-continue
behavior and help/version exits; no new `--` end-of-options state is added.
Managed GUI launches stay in the foreground for the common Host; unmanaged
daemonization, MCP precedence and thumbnail early exits remain unchanged.
Selectors do not grant thumbnail, media-file or audio authority. Managed
delivery requires the matching OS resources and their actual acceptance;
see [native payloads](../../docs/native-payloads.md).
