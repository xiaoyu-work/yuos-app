# Desktop Widgets Product

Own the native and Web Widget Rail presentation of OS resource telemetry.
Widget Rail no longer reads Calendar events or Agent tasks. It does not own
telemetry authority or persistent state.

| Path | Responsibility |
| --- | --- |
| `apps/widget-rail/` | Existing identity, manifest-bound native GUI/MCP and separate observer/panel needs |
| `native/yuos-applet-widget-rail/src/app.rs` | System Health and resource Suggestions cards |
| `native/yuos-applet-widget-rail/src/provider.rs` | Typed read-only telemetry callback and presentation records |
| `native/yuos-applet-widget-rail/src/main.rs`, `src/service.rs`, `src/mcp.rs` | MCP-first entry and shared read-only OS SDK client |
| `native/yuos-applet-widget-rail/build.rs`, `justfile` | Product-owned localized desktop generation and binary installation |
| `native/yuos-applet-widget-rail/i18n/`, `data/` | Original UI/desktop translations and desktop entry |
| `native/yuos-applet-widget-rail/test/unit/` | Original rendering/state helpers and provider/update regressions |
| `native/test_build.py` | Complete native staging and independent build contract |
| `native/Cargo.lock`, `../../tools/native_build.py` | Locked native build against the immutable shared toolkit |
| `PROVENANCE.md`, `native/LICENSE` | Preserved origin and GPL terms |

The published SDK wire client invokes the fixed `/usr/local/bin/yu` binary
with `--wire=1 __observe resources`, using only `sys.observe:Name(resources)`.
The OS authenticates the owner and returns `{summary: SystemSummary}`. The
Root host owns telemetry observation and
sampling; nullable first CPU/network samples remain unavailable until reported,
and subsequent samples and the OS `fallback` flag are preserved unchanged.
There is no fallback to `sys resources`, no local `/proc` sampler,
copied provider, Desktop-only helper or fabricated telemetry. No live data is moved.
Agent execution, task mutations and cross-App orchestration remain OS-owned.
The product imports no other App or OS implementation.

## Read-only MCP and Web view

The same real `bin/yuos-applet-widget-rail` serves the native UI and lazy stdio
MCP. Dispatch precedes GUI initialization and the original direct `--version`
handling; other native arguments retain their ignored behavior.

| Tool | Exact independent need | Result |
| --- | --- | --- |
| `widget-rail.system` | `sys.observe:Name(resources)` | `{summary: SystemSummary}` |

The tool accepts no arguments and does not invoke a model. The `desktop.needs`
requests the telemetry read and separate `ui.panel` capability for native presentation.
Protected owner approval, not widget metadata or read authority, admits the
panel surface. The MCP tool does not request or inherit `ui.panel`.
The standalone entry explicitly creates a top-right Layer Shell surface with
no keyboard interactivity and suppresses the toolkit's default XDG window.
Its 360-by-628 presentation preserves the original 620-pixel scrollable cards
and padding. Geometry and panel metadata supply no display authority.
The OS retains owner/session authorization and state. Calls have a 12-second bound;
cancellation/deadlines drop the SDK CLI future. The Web page presents telemetry
with explicit loading and unavailable states. The native telemetry callback
uses the same client without requiring MCP or an active AI Agent.

The Rust UI uses the same verified App manifest, identity/session, capability
and sandbox contract as every App. Its language, UI and package origin select
no special integration tier. The public data service uses resource scopes,
not a Widget Rail identity exemption or native App allowlist entry.

Both presentations refresh telemetry every five seconds without overlapping
requests. Native suggestions use only reported memory and storage usage.
Calendar events remain in the Calendar product and its panel; task details
remain in the Agent's own UI. Removing these consumers is not OS enforcement.

```bash
python3 tools/test.py desktop-widgets
python3 tools/native_build.py desktop-widgets test
python3 tools/stage.py desktop-widgets --root build/widget-stage
```

The installed identity remains `widget-rail` / `com.yuos.AppletWidgetRail`.
The product's `just install` copies the actual `/usr/bin/yuos-applet-widget-rail`
ELF and generated desktop entry. `native_payload` preserves that ELF, resources
and original GPL license inside the package-local App; source staging never
fabricates a runnable binary. Compatibility launchers use the common Host.
Canonical native builds select the shared App build helper for the exact
current Cargo-reported desktop entry; custom target directories and Cargo
output-layout changes do not select a guessed or stale `xdgen` file.
The binary requires the `yuos-observation-v1 (= 1)` interface.
`cosmic-applets` reads none of its source/assets. The recipe requires an explicit non-root staging
`rootdir`; it installs files, never permissions or installed-system hooks.
Original UI tests remain and the integration test
runs the real binary headlessly. Synthetic CLI/MCP fixtures cover the declared
reads and independent denials without real services or accounts. These checks do not replace interactive
Wayland or full-image acceptance.

Before the Calendar and Agent task cards were removed, the clean-CI Widget Rail lane at OS source
`6024681b09b1df921c2c5015549051cc7dd3bc55`, using immutable App source
`4aa63d3c9abc765568db88a3ae6dddd4489ce7e3`, verified the original signed
standalone panel, three independent owner-scoped SDK reads and checked
process/cgroup/runtime/window/PAM retirement without a model. Protected
same-instance SHM bytes were inspected directly: the themed header, Calendar
event, empty task state, memory/storage and deterministic suggestions were
readable. CPU/network warm-up retained the OS's unavailable first samples.
This closes that bounded read-only UI/resource gate, not hardware/default-WGPU,
other native surfaces or full release readiness.
