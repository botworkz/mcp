#![cfg(unix)]

use botwork_mcp_glint::{init_logging, serve_with_listener};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt};
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

fn extract_jsonrpc_sse_payload(body: &str) -> serde_json::Value {
    for line in body.lines().rev() {
        if let Some(json) = line.strip_prefix("data: ").map(str::trim) {
            if let Ok(value) = serde_json::from_str(json) {
                return value;
            }
        }
    }
    panic!("SSE payload does not include a valid data line")
}

fn build_fake_glint() -> anyhow::Result<(TempDir, String)> {
    let dir = tempfile::tempdir()?;
    let script_path = dir.path().join("fake-glint.sh");

    fs::write(
        &script_path,
        "#!/bin/sh\necho '{\"files\":{},\"summary\":{\"total_files\":0,\"files_with_issues\":0,\"total_issues\":0}}'\n",
    )?;

    let mut perms = fs::metadata(&script_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms)?;

    Ok((dir, script_path.to_string_lossy().to_string()))
}

#[tokio::test]
async fn initialize_and_glint_tools_roundtrip() -> anyhow::Result<()> {
    init_logging();

    let (_tmp, fake_glint) = build_fake_glint()?;
    unsafe {
        std::env::set_var("GLINT_BIN", &fake_glint);
    }

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let shutdown = CancellationToken::new();
    let child_shutdown = shutdown.child_token();

    let server = tokio::spawn(async move { serve_with_listener(listener, child_shutdown).await });

    let client = reqwest::Client::new();
    let url = format!("http://{addr}/mcp");

    let initialize = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {"name": "integration-test", "version": "1.0"}
                }
            })
            .to_string(),
        )
        .send()
        .await?;

    assert_eq!(initialize.status(), 200);

    let session_id = initialize
        .headers()
        .get("mcp-session-id")
        .and_then(|value| value.to_str().ok())
        .expect("initialize response should include mcp-session-id")
        .to_string();

    let init_body = initialize.text().await?;
    let init_json = extract_jsonrpc_sse_payload(&init_body);
    assert_eq!(init_json["id"], 1);

    let initialized = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", session_id.clone())
        .body(
            json!({
                "jsonrpc": "2.0",
                "method": "notifications/initialized",
                "params": {}
            })
            .to_string(),
        )
        .send()
        .await?;

    assert_eq!(initialized.status(), 202);

    let expected = json!({
        "files": {},
        "summary": {
            "total_files": 0,
            "files_with_issues": 0,
            "total_issues": 0
        }
    })
    .to_string();

    for (id, name) in [(2, "glint_check"), (3, "glint_fix")] {
        let tool_call = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("mcp-session-id", session_id.clone())
            .body(
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "method": "tools/call",
                    "params": {
                        "name": name,
                        "arguments": {
                            "paths": ["."]
                        }
                    }
                })
                .to_string(),
            )
            .send()
            .await?;

        assert_eq!(tool_call.status(), 200);
        let tool_body = tool_call.text().await?;
        let tool_json = extract_jsonrpc_sse_payload(&tool_body);

        assert_eq!(tool_json["id"], id);
        assert_eq!(tool_json["result"]["content"][0]["text"], expected);
    }

    shutdown.cancel();
    server.await??;

    unsafe {
        std::env::remove_var("GLINT_BIN");
    }

    Ok(())
}
