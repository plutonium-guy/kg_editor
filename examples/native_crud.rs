use kg_core::schema::{NodeSchema, PropType, SchemaRegistry};
use kg_core::value::PropValue;
use kg_neo4j::{auth::basic, ClientBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reg = SchemaRegistry::new();
    reg.add_node(
        NodeSchema::builder("Person")
            .prop("name", PropType::String).required()
            .unique(["name"])
            .build(),
    );

    let client = ClientBuilder::new("bolt://localhost:7687")
        .auth(basic("neo4j", "test"))
        .schema(reg)
        .build()
        .await?;

    client.materialize_schema().await?;

    let mut uow = client.unit_of_work();
    let alice = uow.create_node(["Person"], [("name", PropValue::from("Alice"))]);
    let bob   = uow.create_node(["Person"], [("name", PropValue::from("Bob"))]);
    uow.create_rel(alice, bob, "KNOWS", [("since", PropValue::Int(2020))]);
    client.commit(uow).await?;

    let rows: Vec<_> = client.query::<std::collections::BTreeMap<String, PropValue>>(
        "MATCH (a:Person)-[r:KNOWS]->(b:Person) RETURN a.name AS a, b.name AS b",
        Vec::<(String, PropValue)>::new(),
    ).await?;
    for row in rows {
        println!("{:?} -KNOWS-> {:?}", row.get("a"), row.get("b"));
    }
    Ok(())
}
