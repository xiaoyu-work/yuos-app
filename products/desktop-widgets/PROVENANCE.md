# Desktop Widgets provenance

The `widget-rail` manifest/launcher and full `yuos-applet-widget-rail` UI,
localization, desktop entry and UI tests were moved from Yu OS commit
`c0198e5f5380ff05c65871bf0cfd4c225027194f`.
The native code is a GPL-3.0-only Yu OS addition to the COSMIC applets fork,
originally based on `pop-os/cosmic-applets` commit `89a149034d06`.
The original GPL license is retained verbatim in [native/LICENSE](native/LICENSE)
and all source SPDX notices are preserved.

Policy, telemetry collection and their provider tests remain in the OS
shared service library. The standalone
main retains the UI/callback seam and uses the public SDK's versioned
fixed helper instead of a statically linked OS host. OS-side telemetry sampling
state remains in a persistent helper; no provider, Agent Activity implementation
or other App is imported. The installed identity and live user state are unchanged.

The native runner uses only the immutable shared toolkit from
[platform.lock.json](../../platform.lock.json), preserving the existing fork
and its licenses, plus the declared public SDK; see
[tools/native_build.py](../../tools/native_build.py). The independent native App
package ships its real ELF and original resources and requires
`yuos-observation-v1 (= 1)`. The OS shell neither links the UI nor reads
its assets.
