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
    panic!("SSE payload does not include a valid data line")
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
    eprintln!("PARSED tool_json: {tool_json:#?}");

    assert_eq!(tool_json["id"], 2);

    // The echo tool now returns a `Json<EchoResponse>` wrapper which rmcp
    // routes into the `structured_content` field on the CallToolResult.
    // `content` still carries a text rendering of the same JSON for clients
    // that haven't migrated, so this test asserts both surfaces.
    let structured = &tool_json["result"]["structuredContent"];
    assert_eq!(
        structured["message"], "hello over mcp",
        "structuredContent.message should round-trip the input"
    );
    assert_eq!(structured["plugin"], "mcp-echo");
    assert!(
        structured["version"].is_string(),
        "structuredContent.version should be present (got {structured:?})"
    );
    assert!(
        structured["env"].is_array(),
        "structuredContent.env should be an array"
    );

    // The text content mirror is documented as containing the same JSON
    // serialised once. The exact string form doesn't matter to clients that
    // parse `structured_content`, but assert it at least mentions the message
    // so a regression to a stringly-typed echo would fail loudly.
    let text = tool_json["result"]["content"][0]["text"]
        .as_str()
        .expect("text content present");
    assert!(
        text.contains("hello over mcp"),
        "text content should include the echoed message: {text}"
    );

    shutdown.cancel();
    server.await??;
    Ok(())
}
