#![cfg(feature = "native")]

use kg_core::value::PropValue;
use kg_neo4j::{auth::basic, ClientBuilder};
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};

/// Spin up a Neo4j 5 Community container and return (uri, container_handle).
async fn spin_neo4j() -> (String, testcontainers::ContainerAsync<GenericImage>) {
    let image = GenericImage::new("neo4j", "5-community")
        .with_exposed_port(7687_u16.tcp())
        .with_wait_for(WaitFor::message_on_stdout("Started."))
        .with_env_var("NEO4J_AUTH", "neo4j/testtest");
    let container = image.start().await.expect("start neo4j container");
    let port = container
        .get_host_port_ipv4(7687_u16.tcp())
        .await
        .expect("get bolt port");
    let uri = format!("bolt://127.0.0.1:{port}");
    (uri, container)
}

async fn neo4j_client() -> (kg_neo4j::Client, testcontainers::ContainerAsync<GenericImage>) {
    let (uri, container) = spin_neo4j().await;
    let client = ClientBuilder::new(&uri)
        .auth(basic("neo4j", "testtest"))
        .build()
        .await
        .expect("connect to neo4j");
    (client, container)
}

async fn neo4j_client_with_schema(
) -> (kg_neo4j::Client, testcontainers::ContainerAsync<GenericImage>) {
    use kg_core::schema::{NodeSchema, PropType, SchemaRegistry};
    let (uri, container) = spin_neo4j().await;
    let mut reg = SchemaRegistry::new();
    // Use unique-only (no `required`) so the schema emits only UNIQUE constraints,
    // which are supported by Neo4j Community Edition. Property-existence constraints
    // require Enterprise Edition and are not tested here.
    reg.add_node(
        NodeSchema::builder("Person")
            .prop("name", PropType::String)
            .unique(["name"])
            .build(),
    );
    let client = ClientBuilder::new(&uri)
        .auth(basic("neo4j", "testtest"))
        .schema(reg)
        .build()
        .await
        .expect("connect");
    (client, container)
}

/// Returns a connected client using an env-supplied URI/password when available
/// (CI service container path), otherwise falls back to spinning up a testcontainers
/// instance (local dev path).
async fn client_via_env_or_container(
) -> (kg_neo4j::Client, Option<testcontainers::ContainerAsync<GenericImage>>) {
    if let (Ok(uri), Ok(pwd)) = (std::env::var("NEO4J_URI"), std::env::var("NEO4J_PASSWORD")) {
        let client = ClientBuilder::new(&uri)
            .auth(basic("neo4j", &pwd))
            .build()
            .await
            .expect("connect via env");
        return (client, None);
    }
    let (client, container) = neo4j_client().await;
    (client, Some(container))
}

/// Like `client_via_env_or_container` but also applies the standard test schema.
async fn client_with_schema_via_env_or_container(
) -> (kg_neo4j::Client, Option<testcontainers::ContainerAsync<GenericImage>>) {
    use kg_core::schema::{NodeSchema, PropType, SchemaRegistry};
    if let (Ok(uri), Ok(pwd)) = (std::env::var("NEO4J_URI"), std::env::var("NEO4J_PASSWORD")) {
        let mut reg = SchemaRegistry::new();
        reg.add_node(
            NodeSchema::builder("Person")
                .prop("name", PropType::String)
                .unique(["name"])
                .build(),
        );
        let client = ClientBuilder::new(&uri)
            .auth(basic("neo4j", &pwd))
            .schema(reg)
            .build()
            .await
            .expect("connect with schema via env");
        return (client, None);
    }
    let (client, container) = neo4j_client_with_schema().await;
    (client, Some(container))
}

#[tokio::test(flavor = "multi_thread")]
async fn create_two_nodes_and_a_rel() {
    let (client, _container) = client_via_env_or_container().await;

    let mut uow = client.unit_of_work();
    let a = uow.create_node(["Person"], [("name", PropValue::from("Alice"))]);
    let b = uow.create_node(["Person"], [("name", PropValue::from("Bob"))]);
    uow.create_rel(a, b, "KNOWS", [("since", PropValue::Int(2020))]);
    client.commit(uow).await.expect("commit");
}

#[tokio::test(flavor = "multi_thread")]
async fn fetch_after_commit() {
    let (client, _c) = client_via_env_or_container().await;
    let rows: Vec<std::collections::BTreeMap<String, kg_core::value::PropValue>> = client
        .query(
            "CREATE (n:Person {name:$name}) RETURN id(n) AS id",
            [("name", kg_core::value::PropValue::from("Carol"))],
        )
        .await
        .unwrap();
    let id = match rows[0].get("id") {
        Some(kg_core::value::PropValue::Int(i)) => *i,
        _ => panic!("expected Int id"),
    };
    let fetched = client
        .fetch_node(kg_core::node::NodeId(id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.labels.first().map(String::as_str), Some("Person"));
}

#[tokio::test(flavor = "multi_thread")]
async fn materialize_constraint_idempotent() {
    let (client, _c) = client_with_schema_via_env_or_container().await;
    client.materialize_schema().await.unwrap();
    client.materialize_schema().await.unwrap(); // idempotent
}
