#![cfg(feature = "native")]

use kg_core::value::PropValue;
use kg_neo4j::{auth::basic, ClientBuilder};
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};

async fn neo4j_client(
) -> (
    kg_neo4j::Client,
    testcontainers::ContainerAsync<GenericImage>,
) {
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
    let client = ClientBuilder::new(&uri)
        .auth(basic("neo4j", "testtest"))
        .build()
        .await
        .expect("connect to neo4j");
    (client, container)
}

#[tokio::test(flavor = "multi_thread")]
async fn create_two_nodes_and_a_rel() {
    let (client, _container) = neo4j_client().await;

    let mut uow = client.unit_of_work();
    let a = uow.create_node(["Person"], [("name", PropValue::from("Alice"))]);
    let b = uow.create_node(["Person"], [("name", PropValue::from("Bob"))]);
    uow.create_rel(a, b, "KNOWS", [("since", PropValue::Int(2020))]);
    client.commit(uow).await.expect("commit");
}
