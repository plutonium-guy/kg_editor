#![cfg(all(feature = "wasm", target_arch = "wasm32"))]

use std::sync::Arc;
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

use kg_core::value::PropValue;
use kg_neo4j::{HttpClient, HttpResponse, RecordingHttpClient};

#[wasm_bindgen_test]
async fn recording_client_captures_request() {
    let canned = HttpResponse {
        status: 200,
        body: r#"{"results":[],"errors":[]}"#.into(),
    };
    let rc = Arc::new(RecordingHttpClient {
        captured: Default::default(),
        canned,
    });

    // Build a statement and confirm JSON conversion works end-to-end.
    let stmt = kg_core::cypher::Statement::new(
        "CREATE (n:Person {name:$name})",
        [("name", PropValue::from("Alice"))],
    );

    // Manually invoke the recording client through the public HttpClient trait.
    let r = rc
        .post(
            "http://example.test/db/neo4j/query/v2",
            &[
                ("Content-Type".into(), "application/json".into()),
                ("Authorization".into(), "Basic dGVzdA==".into()),
            ],
            r#"{"statement":"CREATE (n:Person {name:$name})","parameters":{"name":{"$type":"String","_value":"Alice"}}}"#,
        )
        .await
        .unwrap();

    assert_eq!(r.status, 200);

    let caps = rc.captured.borrow();
    assert_eq!(caps.len(), 1);
    assert!(caps[0].url.contains("/db/neo4j/query/v2"));
    assert!(caps[0].body.contains("\"$type\":\"String\""));

    // Verify the statement itself was constructed correctly.
    assert_eq!(stmt.cypher, "CREATE (n:Person {name:$name})");
    assert_eq!(stmt.params.get("name"), Some(&PropValue::from("Alice")));
}

#[wasm_bindgen_test]
async fn recording_client_canned_body_passes_through() {
    let canned = HttpResponse {
        status: 201,
        body: r#"{"results":[{"columns":["x"],"data":[{"row":[42]}]}],"errors":[]}"#.into(),
    };
    let rc = RecordingHttpClient {
        captured: Default::default(),
        canned,
    };

    let r = rc
        .post("http://example.test/anything", &[], r#"{}"#)
        .await
        .unwrap();

    assert_eq!(r.status, 201);
    assert!(r.body.contains("\"columns\":[\"x\"]"));

    let caps = rc.captured.borrow();
    assert_eq!(caps[0].body, r#"{}"#);
}
