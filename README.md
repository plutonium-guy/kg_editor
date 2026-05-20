# kg_editor

Rust framework for Neo4j 5.x graph CRUD + linking. Phase 0 ships a pure-logic
core (`kg-core`) and a Neo4j transport (`kg-neo4j`) that compiles natively
(Bolt via `neo4rs`) or to `wasm32-unknown-unknown` (HTTP Query API v2 via
`web-sys::fetch`).

## At a glance

```rust
use kg_core::{schema::{NodeSchema, PropType, SchemaRegistry}, value::PropValue};
use kg_neo4j::{auth::basic, ClientBuilder};

let mut reg = SchemaRegistry::new();
reg.add_node(NodeSchema::builder("Person")
    .prop("name", PropType::String).required().unique(["name"]).build());

let client = ClientBuilder::new("bolt://localhost:7687")
    .auth(basic("neo4j","test"))
    .schema(reg)
    .build().await?;
client.materialize_schema().await?;

let mut uow = client.unit_of_work();
let alice = uow.create_node(["Person"], [("name", PropValue::from("Alice"))]);
let bob   = uow.create_node(["Person"], [("name", PropValue::from("Bob"))]);
uow.create_rel(alice, bob, "KNOWS", [] as [(String, PropValue); 0]);
client.commit(uow).await?;
```

## Crates

| Crate       | Purpose                                                          |
|-------------|------------------------------------------------------------------|
| `kg-core`   | Pure types, schema, Unit-of-Work staging, Cypher emitter, no I/O |
| `kg-neo4j`  | Transports (Bolt or HTTP) and `Client` facade                    |

## Build matrix

```
cargo test  -p kg-core
cargo test  -p kg-neo4j --features native              # needs make neo4j-up + Docker
wasm-pack test --headless --chrome crates/kg-neo4j --no-default-features --features wasm
```

## Docs

`cargo doc --no-deps --all-features --open`

See:
- [Design spec](docs/superpowers/specs/2026-05-19-graph-editor-framework-design.md)
- [Implementation plan](docs/superpowers/plans/2026-05-19-kg-editor-framework.md)
- [Manual WASM smoke](docs/manual-smoke.md)

## License

MIT OR Apache-2.0
