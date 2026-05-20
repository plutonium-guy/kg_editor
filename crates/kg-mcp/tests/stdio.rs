use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::Command;

/// Spawn a mock kg-server that responds to POST /entities with a fixed entity.
async fn spawn_mock() -> String {
    let app = axum::Router::new().route(
        "/entities",
        axum::routing::post(|axum::Json(_v): axum::Json<serde_json::Value>| async {
            axum::Json(serde_json::json!({"id": 42, "label": "Person", "props": {}}))
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test(flavor = "multi_thread")]
async fn create_entity_tool_works_over_stdio() {
    let mock_base = spawn_mock().await;

    let mut child = Command::new(env!("CARGO_BIN_EXE_kg-mcp"))
        .env("KG_SERVER_URL", &mock_base)
        .env("RUST_LOG", "warn")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn kg-mcp");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    // 1. initialize
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#;
    stdin.write_all(init.as_bytes()).await.unwrap();
    stdin.write_all(b"\n").await.unwrap();
    stdin.flush().await.unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    assert!(line.contains("\"id\":1"), "expected id:1 in init response, got: {line}");
    assert!(
        line.contains("\"result\""),
        "expected result field in init response, got: {line}"
    );

    // 2. initialized notification — no response expected from server
    stdin
        .write_all(
            b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
        )
        .await
        .unwrap();
    stdin.flush().await.unwrap();

    // 3. tools/call create_entity
    let call = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"create_entity","arguments":{"label":"Person","props":{"name":"Alice"}}}}"#;
    stdin.write_all(call.as_bytes()).await.unwrap();
    stdin.write_all(b"\n").await.unwrap();
    stdin.flush().await.unwrap();

    line.clear();
    reader.read_line(&mut line).await.unwrap();

    // The response should be a JSON-RPC result with id 2.
    // The tool returns a String which rmcp wraps as:
    //   {"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"..."}],"isError":false}}
    // The text payload from the mock contains id:42 and label:"Person".
    assert!(line.contains("\"id\":2"), "expected id:2 in tool-call response, got: {line}");
    assert!(
        line.contains("\"result\""),
        "expected result field in tool-call response, got: {line}"
    );
    assert!(
        line.contains("42") || line.contains("Person"),
        "expected mock payload (id 42 or label Person) in tool-call response, got: {line}"
    );

    let _ = child.kill().await;
}
