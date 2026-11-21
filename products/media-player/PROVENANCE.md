# Media Player provenance

The complete 110-file Yu OS native fork was copied from
`xiaoyu-work/Yu-OS` commit
`1c7756e7ce076ff80e8f94a6c506c10212a25f60`, `desktop/player/`,
together with the `apps/cosmic-player/app.json` descriptor.

Upstream is `https://github.com/pop-os/cosmic-player`, originally vendored
at `d1f63c570c76`. Original GPL-3.0-only license and copyright notices,
all 72 Fluent locales, resource images, build/debian/native configuration,
standalone Cargo lock and optional dependency graph are preserved.
`native/LICENSE` is identical to `native/yuos-player/LICENSE`.

Yu OS changes replace arbitrary-player MCP routing with the typed,
owner-bound OS media adapter; correct the MPRIS name and native Stop/status
behavior; and factor native command/state publication for shared headless
fixtures. The original GStreamer/libcosmic UI and rendering graph are not
replaced with a shell applet. Native configurations, user media and playback
history are not copied.
Two whitespace-only import defects in the upstream PR template and Nix shell
were normalized for repository diff checks; their content and behavior remain.

The public desktop identity remains `com.yuos.Player`. The independent native
App uses `yuos-player` for its App ID, crate and binary. System76/COSMIC
trademarks are not licensed for unrestricted reuse.
