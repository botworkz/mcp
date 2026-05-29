use botwork_mcp_echo::{init_logging, serve_with_listener};
use serde_json::json;
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
    panic!("SSE payload includes a data line")
}

#[tokio::test]
async fn initialize_and_echo_tool_call_roundtrip() -> anyhow::Result<()> {
    init_logging();

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

    let tool_call = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", session_id)
        .body(
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "echo",
                    "arguments": {
                        "message": "hello over mcp"
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

    assert_eq!(tool_json["id"], 2);
    assert_eq!(tool_json["result"]["content"][0]["text"], "hello over mcp");

    shutdown.cancel();
    server.await??;
    Ok(())
}
