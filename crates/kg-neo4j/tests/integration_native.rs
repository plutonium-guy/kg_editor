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

// ── Helper: extract a single i64 from a query result column ─────────────────

async fn query_i64(
    client: &kg_neo4j::Client,
    cypher: &str,
    params: impl IntoIterator<Item = (impl Into<String>, PropValue)>,
    col: &str,
) -> i64 {
    use std::collections::BTreeMap;
    let rows: Vec<BTreeMap<String, PropValue>> =
        client.query(cypher, params).await.expect("query failed");
    match rows.first().and_then(|r| r.get(col)) {
        Some(PropValue::Int(i)) => *i,
        other => panic!("expected Int in column `{col}`, got {other:?}"),
    }
}

// ── Existing tests ────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn create_two_nodes_and_a_rel() {
    use std::collections::BTreeMap;

    let (client, _container) = client_via_env_or_container().await;

    let mut uow = client.unit_of_work();
    let a = uow.create_node(["Person"], [("name", PropValue::from("Alice"))]);
    let b = uow.create_node(["Person"], [("name", PropValue::from("Bob"))]);
    uow.create_rel(a, b, "KNOWS", [("since", PropValue::Int(2020))]);
    client.commit(uow).await.expect("commit");

    let counts: Vec<BTreeMap<String, PropValue>> = client.query(
        "MATCH (p:Person) WITH count(p) AS n_count \
         MATCH ()-[r:KNOWS]->() RETURN n_count, count(r) AS r_count",
        Vec::<(String, PropValue)>::new(),
    ).await.unwrap();
    assert_eq!(counts.len(), 1);
    assert_eq!(counts[0].get("n_count"), Some(&PropValue::Int(2)));
    assert_eq!(counts[0].get("r_count"), Some(&PropValue::Int(1)));
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

// ── New tests ─────────────────────────────────────────────────────────────────

/// 1. merge_node_idempotent
/// Calls merge_node with the same key twice in separate transactions.
/// Afterward exactly one Person named "Eve" must exist with age=30.
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "native")]
async fn merge_node_idempotent() {
    use std::collections::BTreeMap;

    let (client, _c) = client_via_env_or_container().await;

    // First merge
    let mut uow1 = client.unit_of_work();
    uow1.merge_node(
        ["Person"],
        [("name", PropValue::from("Eve"))],
        [("age", PropValue::Int(30))],
    );
    client.commit(uow1).await.expect("first merge commit");

    // Second merge — same key props, same set props
    let mut uow2 = client.unit_of_work();
    uow2.merge_node(
        ["Person"],
        [("name", PropValue::from("Eve"))],
        [("age", PropValue::Int(30))],
    );
    client.commit(uow2).await.expect("second merge commit");

    // Verify exactly one Eve exists with age=30
    let rows: Vec<BTreeMap<String, PropValue>> = client
        .query(
            "MATCH (p:Person {name: 'Eve'}) RETURN count(p) AS cnt, p.age AS age",
            [] as [(String, PropValue); 0],
        )
        .await
        .expect("count query");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("cnt"), Some(&PropValue::Int(1)), "expected exactly one Eve");
    assert_eq!(rows[0].get("age"), Some(&PropValue::Int(30)), "age should be 30");
}

/// 2. update_node_set_and_unset
/// Creates a Person with extra="initial" and age=10.
/// A second tx patches age→11 and removes extra.
/// Verifies age==11 and extra key absent.
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "native")]
async fn update_node_set_and_unset() {
    use kg_core::{node::NodeId, uow::PropPatch};

    let (client, _c) = client_via_env_or_container().await;

    // Create node with extra prop
    let node_id = query_i64(
        &client,
        "CREATE (n:Person {name: 'Frank', extra: 'initial', age: 10}) RETURN id(n) AS id",
        [] as [(String, PropValue); 0],
        "id",
    )
    .await;

    // Apply patch: set age=11, unset extra
    let patch = PropPatch::new()
        .set("age", PropValue::Int(11))
        .unset("extra");
    let mut uow = client.unit_of_work();
    uow.update_node(NodeId(node_id), patch);
    client.commit(uow).await.expect("patch commit");

    // Fetch and verify
    let fetched = client
        .fetch_node(NodeId(node_id))
        .await
        .expect("fetch_node error")
        .expect("node not found");

    assert_eq!(
        fetched.props.get("age"),
        Some(&PropValue::Int(11)),
        "age should be 11"
    );
    assert!(
        !fetched.props.contains_key("extra"),
        "extra key should have been removed"
    );
}

/// 3. delete_node_detach
/// Creates two Person nodes connected by KNOWS.
/// Deletes one with CascadeRule::Detach.
/// Verifies the deleted node and rel are gone; the other node survives.
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "native")]
async fn delete_node_detach() {
    use kg_core::{node::NodeId, uow::CascadeRule};
    use std::collections::BTreeMap;

    let (client, _c) = client_via_env_or_container().await;

    // Create two nodes and a rel
    let rows: Vec<BTreeMap<String, PropValue>> = client
        .query(
            "CREATE (a:Person {name:'Hana'})-[:KNOWS]->(b:Person {name:'Ivan'}) \
             RETURN id(a) AS aid, id(b) AS bid",
            [] as [(String, PropValue); 0],
        )
        .await
        .expect("create");
    let aid = match rows[0].get("aid") { Some(PropValue::Int(i)) => *i, _ => panic!() };
    let bid = match rows[0].get("bid") { Some(PropValue::Int(i)) => *i, _ => panic!() };

    // Delete 'a' with DETACH
    let mut uow = client.unit_of_work();
    uow.delete_node(NodeId(aid), CascadeRule::Detach);
    client.commit(uow).await.expect("delete commit");

    // 'a' should be gone
    let a_gone = client.fetch_node(NodeId(aid)).await.expect("fetch a");
    assert!(a_gone.is_none(), "node 'a' should have been deleted");

    // 'b' should still exist
    let b_exists = client.fetch_node(NodeId(bid)).await.expect("fetch b");
    assert!(b_exists.is_some(), "node 'b' should still exist");

    // Rel should be gone
    let rel_count: Vec<BTreeMap<String, PropValue>> = client
        .query(
            "MATCH (a)-[r:KNOWS]->(b) WHERE id(b) = $bid RETURN count(r) AS cnt",
            [("bid", PropValue::Int(bid))],
        )
        .await
        .expect("rel count");
    assert_eq!(rel_count[0].get("cnt"), Some(&PropValue::Int(0)), "rel should be gone");
}

/// 4. delete_node_strict_with_rel_fails
/// Creates two connected Person nodes.
/// Attempts to delete one with CascadeRule::Strict (no DETACH).
/// Expects an Err from commit because Neo4j refuses to delete a node with relations.
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "native")]
async fn delete_node_strict_with_rel_fails() {
    use kg_core::{node::NodeId, uow::CascadeRule};
    use kg_neo4j::Neo4jError;
    use std::collections::BTreeMap;

    let (client, _c) = client_via_env_or_container().await;

    // Create two connected nodes
    let rows: Vec<BTreeMap<String, PropValue>> = client
        .query(
            "CREATE (a:Person {name:'Jake'})-[:KNOWS]->(b:Person {name:'Kate'}) \
             RETURN id(a) AS aid",
            [] as [(String, PropValue); 0],
        )
        .await
        .expect("create");
    let aid = match rows[0].get("aid") { Some(PropValue::Int(i)) => *i, _ => panic!() };

    // Attempt strict delete — should fail
    let mut uow = client.unit_of_work();
    uow.delete_node(NodeId(aid), CascadeRule::Strict);
    let result = client.commit(uow).await;

    // Neo4j 5 Community wraps the "Cannot delete node, because it still has relationships"
    // error as a protocol-level failure (Neo.ClientError.Schema.ConstraintValidationFailed or
    // Neo.DatabaseError.Transaction.TransactionHookFailed).
    // The bolt transport surfaces this as Neo4jError::Transport(TransportError::Protocol(_)).
    assert!(
        result.is_err(),
        "expected commit to fail for strict delete of node with rels, got Ok"
    );
    match result.unwrap_err() {
        Neo4jError::Transport(kg_neo4j::TransportError::Protocol(_)) => { /* expected */ }
        other => panic!(
            "expected Neo4jError::Transport(TransportError::Protocol(_)), got: {other:?}"
        ),
    }
}

/// 5. update_rel_props
/// Creates two nodes + KNOWS rel with since=2020.
/// A second tx runs update_rel with since→2021 and adds strength=0.9.
/// Fetches the rel and verifies both props.
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "native")]
async fn update_rel_props() {
    use kg_core::{rel::RelId, uow::PropPatch};
    use std::collections::BTreeMap;

    let (client, _c) = client_via_env_or_container().await;

    // Create two nodes + rel
    let rows: Vec<BTreeMap<String, PropValue>> = client
        .query(
            "CREATE (a:Person {name:'Lena'})-[r:KNOWS {since: 2020}]->(b:Person {name:'Mike'}) \
             RETURN id(r) AS rid",
            [] as [(String, PropValue); 0],
        )
        .await
        .expect("create");
    let rid = match rows[0].get("rid") { Some(PropValue::Int(i)) => *i, _ => panic!() };

    // Update rel props
    let patch = PropPatch::new()
        .set("since", PropValue::Int(2021))
        .set("strength", PropValue::Float(0.9));
    let mut uow = client.unit_of_work();
    uow.update_rel(RelId(rid), patch);
    client.commit(uow).await.expect("update rel commit");

    // Fetch and verify
    let rel = client
        .fetch_rel(RelId(rid))
        .await
        .expect("fetch_rel error")
        .expect("rel not found");

    assert_eq!(rel.props.get("since"), Some(&PropValue::Int(2021)), "since should be 2021");
    match rel.props.get("strength") {
        Some(PropValue::Float(f)) => {
            assert!((f - 0.9).abs() < 1e-9, "strength should be 0.9, got {f}");
        }
        other => panic!("expected Float for strength, got {other:?}"),
    }
}

/// 6. delete_rel_only
/// Creates two nodes + KNOWS rel.
/// Deletes just the rel.
/// Verifies both nodes still exist and the rel is gone.
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "native")]
async fn delete_rel_only() {
    use kg_core::{node::NodeId, rel::RelId};
    use std::collections::BTreeMap;

    let (client, _c) = client_via_env_or_container().await;

    // Create two nodes + rel
    let rows: Vec<BTreeMap<String, PropValue>> = client
        .query(
            "CREATE (a:Person {name:'Nina'})-[r:KNOWS {since:2022}]->(b:Person {name:'Omar'}) \
             RETURN id(a) AS aid, id(b) AS bid, id(r) AS rid",
            [] as [(String, PropValue); 0],
        )
        .await
        .expect("create");
    let aid = match rows[0].get("aid") { Some(PropValue::Int(i)) => *i, _ => panic!() };
    let bid = match rows[0].get("bid") { Some(PropValue::Int(i)) => *i, _ => panic!() };
    let rid = match rows[0].get("rid") { Some(PropValue::Int(i)) => *i, _ => panic!() };

    // Delete only the rel
    let mut uow = client.unit_of_work();
    uow.delete_rel(RelId(rid));
    client.commit(uow).await.expect("delete rel commit");

    // Both nodes still exist
    let a = client.fetch_node(NodeId(aid)).await.expect("fetch a");
    assert!(a.is_some(), "node 'a' should still exist");
    let b = client.fetch_node(NodeId(bid)).await.expect("fetch b");
    assert!(b.is_some(), "node 'b' should still exist");

    // Rel is gone
    let rel = client.fetch_rel(RelId(rid)).await.expect("fetch rel");
    assert!(rel.is_none(), "rel should have been deleted");
}

/// 7. merge_rel_idempotent
/// Creates Alice and Bob via merge_node.
/// Runs the same merge_rel tx twice.
/// Verifies exactly one KNOWS rel between them.
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "native")]
async fn merge_rel_idempotent() {
    use kg_core::node::{NodeId, NodeRef};
    use std::collections::BTreeMap;

    let (client, _c) = client_via_env_or_container().await;

    // Create Alice and Bob (separate transactions so we get their server ids)
    let alice_id = query_i64(
        &client,
        "MERGE (p:Person {name:'Alice2'}) RETURN id(p) AS id",
        [] as [(String, PropValue); 0],
        "id",
    )
    .await;
    let bob_id = query_i64(
        &client,
        "MERGE (p:Person {name:'Bob2'}) RETURN id(p) AS id",
        [] as [(String, PropValue); 0],
        "id",
    )
    .await;

    // First merge_rel
    let mut uow1 = client.unit_of_work();
    uow1.merge_rel(
        NodeRef::Server(NodeId(alice_id)),
        NodeRef::Server(NodeId(bob_id)),
        "KNOWS",
        [("since", PropValue::Int(2020))],
        [("strength", PropValue::Float(0.5))],
    );
    client.commit(uow1).await.expect("first merge_rel commit");

    // Second merge_rel — identical
    let mut uow2 = client.unit_of_work();
    uow2.merge_rel(
        NodeRef::Server(NodeId(alice_id)),
        NodeRef::Server(NodeId(bob_id)),
        "KNOWS",
        [("since", PropValue::Int(2020))],
        [("strength", PropValue::Float(0.5))],
    );
    client.commit(uow2).await.expect("second merge_rel commit");

    // Exactly one KNOWS rel
    let rows: Vec<BTreeMap<String, PropValue>> = client
        .query(
            "MATCH (a:Person {name:'Alice2'})-[r:KNOWS]->(b:Person {name:'Bob2'}) \
             RETURN count(r) AS cnt",
            [] as [(String, PropValue); 0],
        )
        .await
        .expect("count rels");
    assert_eq!(
        rows[0].get("cnt"),
        Some(&PropValue::Int(1)),
        "expected exactly one KNOWS rel"
    );
}

/// 8. fetch_rel_returns_rel
/// Creates two nodes + KNOWS rel with known props.
/// Captures the rel server id via a follow-up query.
/// Calls client.fetch_rel(RelId(id)) and asserts the returned rel has correct
/// type, props, and Server variant endpoints.
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "native")]
async fn fetch_rel_returns_rel() {
    use kg_core::{node::NodeRef, rel::RelId};
    use std::collections::BTreeMap;

    let (client, _c) = client_via_env_or_container().await;

    // Create the graph
    let rows: Vec<BTreeMap<String, PropValue>> = client
        .query(
            "CREATE (a:Person {name:'Pat'})-[r:KNOWS {since:1999, label:'old friends'}]\
             ->(b:Person {name:'Quinn'}) RETURN id(r) AS rid",
            [] as [(String, PropValue); 0],
        )
        .await
        .expect("create");
    let rid = match rows[0].get("rid") { Some(PropValue::Int(i)) => *i, _ => panic!() };

    // Fetch and assert
    let rel = client
        .fetch_rel(RelId(rid))
        .await
        .expect("fetch_rel error")
        .expect("rel not found");

    assert_eq!(rel.r#type, "KNOWS", "rel type mismatch");
    assert_eq!(rel.props.get("since"), Some(&PropValue::Int(1999)));
    assert_eq!(
        rel.props.get("label"),
        Some(&PropValue::String("old friends".into()))
    );
    assert!(
        matches!(rel.start, NodeRef::Server(_)),
        "start should be Server variant"
    );
    assert!(
        matches!(rel.end, NodeRef::Server(_)),
        "end should be Server variant"
    );
    assert!(rel.id.is_some(), "rel id should be Some");
    assert_eq!(rel.id, Some(RelId(rid)));
}

/// 9. server_endpoint_rel_creation
/// Exercises Local+Server mixing in a single UoW transaction.
/// Alice is created via raw Cypher (Server endpoint); Bob is created via
/// UoW create_node (Local). A rel is created from Server(alice) to Local(bob)
/// in the same UoW. With the WITH * chaining fix, variable bindings persist
/// across clauses so this now works correctly.
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "native")]
async fn server_endpoint_rel_creation() {
    use kg_core::node::{NodeId, NodeRef};
    use std::collections::BTreeMap;

    let (client, _c) = client_via_env_or_container().await;

    // Create Alice via raw Cypher; capture her server id.
    let alice_id = query_i64(
        &client,
        "CREATE (n:Person {name:'AliceS'}) RETURN id(n) AS id",
        [] as [(String, PropValue); 0],
        "id",
    )
    .await;

    // In one UoW: create Bob (Local) and a rel from Server(alice) -> Local(bob).
    // The WITH * fix ensures n_<bob_local> is in scope when the CreateRel clause runs.
    let mut uow = client.unit_of_work();
    let bob_local = uow.create_node(["Person"], [("name", PropValue::from("BobS"))]);
    uow.create_rel(
        NodeRef::Server(NodeId(alice_id)),
        NodeRef::Local(bob_local),
        "KNOWS",
        [("since", PropValue::Int(2025))],
    );
    client.commit(uow).await.expect("rel commit via Local+Server mix");

    // Verify the rel exists and connects Alice → Bob
    let rows: Vec<BTreeMap<String, PropValue>> = client
        .query(
            "MATCH (a:Person {name:'AliceS'})-[r:KNOWS]->(b:Person {name:'BobS'}) \
             RETURN count(r) AS cnt, r.since AS since",
            [] as [(String, PropValue); 0],
        )
        .await
        .expect("verify query");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("cnt"), Some(&PropValue::Int(1)), "expected one KNOWS rel");
    assert_eq!(rows[0].get("since"), Some(&PropValue::Int(2025)), "since should be 2025");
}
