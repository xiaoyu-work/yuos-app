# yuos-sdk (Rust)

The official Rust SDK for Yu OS. Use this crate to talk to the
`yu` kernel CLI from a Rust app — typed, documented, audited.

## Install

In your App's `Cargo.toml`:

```toml
[dependencies]
yuos-sdk = { git = "https://github.com/xiaoyu-work/Yu-OS.git", tag = "yuos-sdk-v0.1.0" }
```

Live calls require the [Yu OS runtime](../quickstart.html#install).

## What's in it

| Module      | Purpose                                                                         |
|-------------|---------------------------------------------------------------------------------|
| `app`       | Command-line and desktop functions for the App.                                  |
| `ai`        | `chat`, `embed`, `tts`, `stt`, `imagegen`, and `decide`. |
| `mcp`       | Manifest-bound App MCP runtime, call context, and stdio transport.                |
| `applet`    | Versioned OS Calendar/task/telemetry client and separate history-permission preflight. |
| `tools`     | `yu ai tool <name>` — explicitly call a catalog operation.                   |
| `gui`       | Desktop GUI bootstrap and kernel-provided launch context.                       |
| `objects`   | Format and parse canonical App object references. |
| `manifest`, `events`, `channels` | Manifest path validation, App events, and authorized App-owned channel controls. |
| `envelope`  | Strict CLI wire-v1 envelope decoder. |
| `generated` | Typed structs generated from `wire/v1/*.schema.json`.                          |

The OS checks capabilities when public SDK operations run.
Use `yuos_sdk::mcp` to expose App tools to the System Agent.

Installed native clients can use `yu_call_json_async_with_binary` with an
absolute OS CLI path and select it against `CallContext::cancelled()`. It uses
the shared wire decoder and kills its CLI child when dropped. Check the
outcome of an accepted mutation before retrying it.

The `call`/`call_typed` transport serves authenticated owner CLI and desktop
clients. Apps use their own capabilities for AI and OS services.

## App data services

`applet::Client::installed()` owns a private, bounded connection to
`/usr/libexec/yuos-applet-provider`. It exposes `calendar_day`,
`calendar_today`, `tasks`, `system` and `require_history`. Clones serialize
requests on one connection; separate clients retain independent sampling and
refresh state. Invalid replies, deadlines and cancellation discard the
connection and stop its owned helper processes without automatically retrying.
`with_binary` selects an explicit absolute development fixture, never a
different identity or authority tier.

See the [versioned App data contract](../wire/v1/applet-services.md) for exact
records, limits and per-operation scopes. History checks are preflight, not a
grant, CopyQ execution or evidence of resource isolation/revocation. Consumers
still need authenticated OS admission and existing independent grants.
This client is part of the unpublished development-platform 0.1.0 source.
The earlier platform releases have been withdrawn. Consumers must not assume
an artifact exists or fabricate a download pin; publication requires an explicit
owner decision.

## Expose manifest-declared MCP tools

Use the [MCP example manifest](../../docs/app-development.md#quickstart-an-mcp-only-app) with `"runtime": "binary"`
and `mcp.entry` set to `server`. Include the compiled Linux executable before
signing. This example also depends on `async-trait`, `serde_json`, and `tokio`
with the `macros` and `rt` features. The manifest declares the tool; Rust binds
its implementation:

```rust,no_run
use std::sync::Arc;

use async_trait::async_trait;
use yuos_sdk::mcp::{App, CallContext, Tool, ToolResult};
use serde_json::Value;

struct Say;

#[async_trait]
impl Tool for Say {
    fn name(&self) -> &str {
        "hello-mcp.say"
    }

    async fn handle(&self, args: Value, context: CallContext) -> ToolResult {
        if let Err(cancelled) = context.check_cancelled() {
            return ToolResult::error(cancelled.to_string());
        }
        match args["message"].as_str() {
            Some(text) => ToolResult::text(text),
            None => ToolResult::error("validated text argument was unavailable"),
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::from_environment()?;
    app.bind(Arc::new(Say))?;
    app.serve_stdio().await?;
    Ok(())
}
```

`App::from_environment()` reads `YUOS_APP_MANIFEST` and falls back to
`app.json`. Calls require the Gateway-authenticated context under
`_meta["yuos.dev/call-context"]`; handlers receive it as an immutable
`CallContext` snapshot with lineage, deadlines, cooperative cancellation, and
optional progress reporting. `CallContext::deadline_unix_ms()` exposes the
authenticated deadline exactly as the wire `u64`; `cancelled()` and
`check_cancelled()` enforce it without converting it to a platform system-time
range.

## Use it

```rust
use yuos_sdk::ai;

fn summarise_email(body: &str) -> Result<String, Box<dyn std::error::Error>> {
    let response = ai::chat(body, ai::ChatOpts::default())?;
    Ok(response.output.text)
}
```

## Command line and desktop

Set `"runtime": "binary"` and register the function for each version the App
declares in `app.json`:

```rust
use yuos_sdk::{ai, app::App};

fn main() -> Result<(), yuos_sdk::app::AppError> {
    App::new()
        .command("summarize", |args| {
            let reply = ai::chat(&args[0], ai::ChatOpts::default())?;
            Ok(serde_json::json!({"summary": reply.output.text}))
        })
        .desktop(|| Ok(())) // Open your window with any GUI toolkit.
        .run()
}
```

See [Command line and desktop](../api.html#rust-mod-app).

## App object references

```rust
use yuos_sdk::{generated::ObjectRef, objects, ObjectRefError};

fn object_reference_example() -> Result<(), ObjectRefError> {
    let reference = ObjectRef {
        app_id: "notes".into(),
        object_type: "note".into(),
        object_id: "draft/1".into(),
        revision: None,
    };
    objects::validate(&reference)?;
    let uri = objects::format_reference(&reference)?;
    assert_eq!(uri, "app://notes/note?id=draft%2F1");
    assert_eq!(objects::parse_reference(&uri)?.object_id, "draft/1");
    Ok(())
}
```

The helpers are pure and use the generated type. They preserve opaque
identifiers, reject noncanonical URIs, and return `ObjectRefError` without
opening storage or invoking an App. A reference is not permission or proof
that data exists. When decoding JSON, run `generated::validate_object_ref`
before deserializing into `ObjectRef`.

See [the shared contract](../wire/v1/object-references.md) for limits and the
optional App manifest `objects` resolver declaration.

## AI support

Use `ai::embed`, `ai::tts`, `ai::stt`, `ai::imagegen`, or `ai::decide` with
`ModelOpts::default()`. All six calls expose `provider`, `model`, `units`, and
`output`. See the [model capabilities](../api.html#rust-mod-ai).
`ai::stt` takes audio bytes; `Speech::audio` and `Image::image` hold the
returned bytes. `ChatOpts` takes `.images(...)` with image bytes and
`.functions(...)`.
`ai::open_permission_settings()`
opens the [model permission page](../../docs/app-development.md#model-permissions-in-settings)
after a user action and with the separate desktop launcher permission.

Match `AiError::Denied`, `BudgetExceeded`, `InvalidArg`, `SafetyViolation`,
or `ServiceUnavailable` for the [AI error codes](../reference.html#handle-ai-errors).
`Unavailable` and `Bridge` also report transport and decoding failures.

## Configuration

| Env var          | Effect                                                                |
|------------------|-----------------------------------------------------------------------|
| `YUOS_KERNEL_BIN`   | Path to the `yu` binary. Defaults to looking up `yu` in `$PATH`.    |
| `YUOS_APP_ID`     | App id, required for `ai::chat` and `tools::call` unless passed via `.app(...)`. |
| `YUOS_APP_MANIFEST` | Authoritative manifest path used by `mcp::App::from_environment()`. |

Installed native clients whose launcher sanitizes `PATH` can call
`yu_call_json_with_binary("/usr/local/bin/yu", family, verb, args)`.
This explicit-binary primitive transport ignores `YUOS_KERNEL_BIN`, changes no
process environment, and shares the ordinary wire/error decoder. Generic SDK
calls retain their existing override/PATH behavior. Neither transport supplies
caller identity, approval or extra capabilities.

## Wire protocol

The [notification presentation companion](notification-presentation/README.md)
is a separate lightweight crate inside the same development export. OS shell
consumers and App presenters share its versioned records/private-connection
client declarations without importing App configuration, image/markup code,
or this crate's broader RPC/JSON feature set. It defines no authority or
provider-selection privilege; durable notification state and DND remain OS-owned.

This crate implements wire protocol v1. See
[`../wire/v1/README.md`](../wire/v1/README.md) for the full spec.
Regenerate typed structs with:

```sh
python3 ../wire/codegen.py
```
