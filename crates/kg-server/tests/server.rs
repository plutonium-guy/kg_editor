use kg_neo4j::{auth::basic, ClientBuilder};
use kg_schema::SchemaFile;
use std::io::Write;
use std::sync::Arc;
use testcontainers::{
    core::{ContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};

async fn spin_neo4j() -> (String, ContainerAsync<GenericImage>) {
    let image = GenericImage::new("neo4j", "5-community")
        .with_exposed_port(ContainerPort::Tcp(7687))
        .with_wait_for(WaitFor::message_on_stdout("Started."))
        .with_env_var("NEO4J_AUTH", "neo4j/testtest");
    let container = image.start().await.expect("neo4j container start");
    let port = container
        .get_host_port_ipv4(ContainerPort::Tcp(7687))
        .await
        .expect("port");
    let uri = format!("bolt://127.0.0.1:{port}");
    (uri, container)
}

async fn start_server() -> (String, Option<ContainerAsync<GenericImage>>) {
    let (uri, password, container) = match (
        std::env::var("NEO4J_URI"),
        std::env::var("NEO4J_PASSWORD"),
    ) {
        (Ok(u), Ok(p)) => (u, p, None),
        _ => {
            let (uri, c) = spin_neo4j().await;
            (uri, "testtest".to_string(), Some(c))
        }
    };
    let client = ClientBuilder::new(&uri)
        .auth(basic("neo4j", &password))
        .build()
        .await
        .expect("connect");

    // Write the example schema to a temp file and parse.
    let yaml = include_str!("../../../kg-schema.yaml");
    let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
    tmp.write_all(yaml.as_bytes()).expect("write yaml");
    let schema = SchemaFile::from_yaml(tmp.path()).expect("parse schema");

    let state = kg_server::state::AppState {
        client: Arc::new(client),
        schema: Arc::new(schema),
    };
    let app = kg_server::router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), container)
}

#[tokio::test(flavor = "multi_thread")]
async fn health_ok() {
    let (base, _c) = start_server().await;
    let r = reqwest::get(format!("{base}/health")).await.unwrap();
    assert!(r.status().is_success());
    let body: serde_json::Value = r.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["neo4j"], "reachable");
}

#[tokio::test(flavor = "multi_thread")]
async fn commit_and_query() {
    let (base, _c) = start_server().await;
    let cli = reqwest::Client::new();

    // PropValue uses #[serde(tag = "kind", content = "value", rename_all = "snake_case")]
    // so string params must be {"kind": "string", "value": "Carol"}
    let body = serde_json::json!({
        "ddl": [],
        "data": {
            "cypher": "CREATE (n:Person {name:$name})",
            "params": {
                "name": { "kind": "string", "value": "Carol" }
            }
        }
    });
    let r = cli
        .post(format!("{base}/commit"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success(), "commit failed: {}", r.status());

    let body = serde_json::json!({
        "cypher": "MATCH (n:Person {name:$name}) RETURN n.name AS name",
        "params": {
            "name": { "kind": "string", "value": "Carol" }
        }
    });
    let r = cli
        .post(format!("{base}/query"))
        .json(&body)
        .send()
        .await
        .unwrap();
    let v: serde_json::Value = r.json().await.unwrap();
    assert_eq!(v["rows"][0]["name"]["value"], "Carol");
}

#[tokio::test(flavor = "multi_thread")]
async fn bad_cypher_returns_422() {
    let (base, _c) = start_server().await;
    let cli = reqwest::Client::new();
    let body = serde_json::json!({
        "cypher": "THIS IS NOT CYPHER",
        "params": {}
    });
    let r = cli
        .post(format!("{base}/query"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status().as_u16(), 422);
    let v: serde_json::Value = r.json().await.unwrap();
    assert!(
        v["error"]["code"].as_str().unwrap().starts_with("422."),
        "expected 422.* error code, got: {}",
        v["error"]["code"]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn schema_returns_yaml() {
    let (base, _c) = start_server().await;
    let r = reqwest::get(format!("{base}/schema")).await.unwrap();
    assert!(r.status().is_success());
    let v: serde_json::Value = r.json().await.unwrap();
    assert!(v["nodes"].get("Person").is_some(), "expected Person in returned schema");
    assert!(v["rels"].get("KNOWS").is_some(), "expected KNOWS in returned schema");
}

#[tokio::test(flavor = "multi_thread")]
async fn create_entity_validates_and_persists() {
    let (base, _c) = start_server().await;
    let cli = reqwest::Client::new();

    let body = serde_json::json!({
        "label": "Person",
        "props": { "name": "Alice", "role": "engineer" }
    });
    let r = cli.post(format!("{base}/entities")).json(&body).send().await.unwrap();
    assert!(r.status().is_success(), "got {}: {}", r.status(), r.text().await.unwrap());
    let v: serde_json::Value = r.json().await.unwrap();
    assert_eq!(v["label"], "Person");
    assert!(v["id"].as_i64().is_some());

    let body = serde_json::json!({"label": "Person", "props": { "role": "engineer" }});
    let r = cli.post(format!("{base}/entities")).json(&body).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 400);

    let body = serde_json::json!({"label": "Person", "props": { "name": "Bob", "role": "wizard" }});
    let r = cli.post(format!("{base}/entities")).json(&body).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 400);
}

#[tokio::test(flavor = "multi_thread")]
async fn update_entity_set_and_unset() {
    let (base, _c) = start_server().await;
    let cli = reqwest::Client::new();
    let r = cli
        .post(format!("{base}/entities"))
        .json(&serde_json::json!({
            "label": "Person",
            "props": {"name": "Eve", "role": "engineer", "age": 30}
        }))
        .send()
        .await
        .unwrap();
    let v: serde_json::Value = r.json().await.unwrap();
    let id = v["id"].as_i64().unwrap();

    let r = cli
        .put(format!("{base}/entities/{id}"))
        .json(&serde_json::json!({
            "set": {"age": 31},
            "unset": ["role"]
        }))
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success(), "update failed: {}", r.status());
    let v: serde_json::Value = r.json().await.unwrap();
    assert_eq!(v["ok"], true);
}
