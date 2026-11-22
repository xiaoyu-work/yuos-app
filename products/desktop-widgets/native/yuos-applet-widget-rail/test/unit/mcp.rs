use super::*;
use yuos_sdk::mcp::{Frame, InMemoryTransport, Transport, in_memory_pair};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, sync::atomic::{AtomicUsize, Ordering}};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = PathBuf::from("test-fixtures").join(format!("widgets-{}-{}",
            std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        fs::write(root.join("yu"), r#"#!/usr/bin/python3
import json, sys
from pathlib import Path
root = Path(__file__).parent
with (root / "calls.jsonl").open("a") as out:
    out.write(json.dumps(sys.argv[1:]) + "\n")
assert sys.argv[1:] == ["--wire=1", "__observe", "resources"], sys.argv
control = json.loads((root / "response.json").read_text())
if control["deny"]:
    print(json.dumps({"wire_version": 1, "ok": False, "error": "synthetic resource read denied",
        "code": "PERMISSION_DENIED"}))
    sys.exit(1)
print(json.dumps({"wire_version": 1, "ok": True, "data": control["data"]}))
"#).unwrap();
        fs::set_permissions(root.join("yu"), fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }

    fn respond(&self, deny: bool) {
        fs::write(self.0.join("response.json"), json!({
            "deny": deny,
            "data": {"summary": {"cpu_percent": null,
                "memory": {"used_mb": 10, "total_mb": 20},
                "storage": {"used_mb": 30, "total_mb": 100},
                "network_down_bps": null, "network_up_bps": null, "fallback": false}}
        }).to_string()).unwrap();
    }

    fn replace(&self, data: Value) {
        let path = self.0.join("response.json");
        let mut control: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        control["data"] = data;
        fs::write(path, control.to_string()).unwrap();
    }

    fn client(&self) -> Client { Client::with_binary(self.0.join("yu")) }

    fn calls(&self) -> usize {
        fs::read_to_string(self.0.join("calls.jsonl")).unwrap_or_default().lines().count()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
        let _ = fs::remove_dir("test-fixtures");
    }
}

async fn request(client: &InMemoryTransport, id: u64, source: &str, arguments: Value, deadline: Option<u64>) -> Value {
    let mut context = json!({
        "wire_version": 1, "call_id": format!("widgets-{id}"), "trace_id": "synthetic-trace",
        "session_id": "synthetic-session",
        "caller": {"kind": "system-agent", "id": "synthetic-agent", "owner_uid": 1000}
    });
    if let Some(deadline) = deadline {
        context["deadline_unix_ms"] = json!(deadline);
    }
    client.send(json!({
        "jsonrpc": "2.0", "id": id, "method": "tools/call",
        "params": {"name": format!("widget-rail.{source}"), "arguments": arguments,
            "_meta": {"yuos.dev/call-context": context}
        }
    }).to_string()).await.unwrap();
    let frame = tokio::time::timeout(std::time::Duration::from_secs(5), client.recv())
        .await.unwrap().unwrap().unwrap();
    let Frame::Message(frame) = frame else { panic!("expected MCP message") };
    serde_json::from_str(&frame).unwrap()
}

#[tokio::test]
async fn only_system_resources_dispatch_and_denied_or_invalid_reads_are_errors() {
    let fixture = Fixture::new();
    fixture.respond(false);
    let app = App::from_manifest(concat!(env!("CARGO_MANIFEST_DIR"), "/app.json")).unwrap();
    let (client, server) = in_memory_pair();
    let serving = tokio::spawn(bind(app, fixture.client()).unwrap().serve(server));
    for (id, source) in [(1, "calendar"), (2, "tasks")] {
        let retired = request(&client, id, source, json!({}), None).await;
        assert!(retired.get("error").is_some() || retired["result"]["isError"] == true, "{retired}");
    }
    assert_eq!(fixture.calls(), 0);
    let system = request(&client, 3, "system", json!({}), None).await;
    let summary = &system["result"]["structuredContent"]["summary"];
    assert_eq!(summary["memory"]["used_mb"], 10);
    assert_eq!(summary["storage"]["total_mb"], 100);
    assert!(summary["cpu_percent"].is_null());
    assert!(summary["network_down_bps"].is_null());
    assert!(summary["network_up_bps"].is_null());
    assert_eq!(summary["fallback"], false, "preserve the OS fallback flag");
    assert_eq!(fixture.calls(), 1);

    let sampled = json!({"cpu_percent": 42.5, "memory": {"used_mb": 11, "total_mb": 20},
        "storage": null, "network_down_bps": 123, "network_up_bps": 456, "fallback": true});
    fixture.replace(json!({"summary": sampled}));
    let system = request(&client, 4, "system", json!({}), None).await;
    assert_eq!(system["result"]["structuredContent"]["summary"], sampled);
    fixture.replace(json!({"memory": {"used_mb": 10, "total_mb": 20}}));
    assert_eq!(request(&client, 5, "system", json!({}), None).await["result"]["isError"], true,
        "the observer must return the typed summary wrapper");

    let invalid = request(&client, 6, "system", json!({"start": true}), None).await;
    assert_eq!(invalid["result"]["isError"], true, "{invalid}");
    assert_eq!(request(&client, 7, "system", json!({}), Some(1)).await["result"]["isError"], true);
    assert_eq!(fixture.calls(), 3);
    fixture.respond(true);
    let denied = request(&client, 8, "system", json!({}), None).await;
    assert_eq!(denied["result"]["isError"], true);
    assert!(denied["result"]["content"][0]["text"].as_str().unwrap().contains("denied"));
    fixture.respond(false);
    assert_eq!(request(&client, 9, "system", json!({}), None).await["result"]["isError"], false);
    drop(client);
    serving.await.unwrap().unwrap();
}

#[test]
fn another_manifest_identity_cannot_bind_the_widget_service() {
    let fixture = Fixture::new();
    let mut manifest: Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/app.json"))).unwrap();
    manifest["id"] = json!("panel-calendar");
    fs::write(fixture.0.join("other.json"), manifest.to_string()).unwrap();
    assert!(matches!(bind(App::from_manifest(fixture.0.join("other.json")).unwrap(), fixture.client()),
        Err(AppError::Manifest(_))));
}
