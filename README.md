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

### Phase 0 (Rust framework)
```
cargo test  -p kg-core
cargo test  -p kg-neo4j --features native
wasm-pack test --headless --chrome crates/kg-neo4j --no-default-features --features wasm
```

### Phase 1 (Browser UI)
```
make neo4j-up
make wasm-build        # builds kg-core-wasm into webui/pkg
make kg-server-run     # starts the HTTP proxy on :9000
make webui-dev         # Vite dev server on :5173
```

See [Phase 1 design](docs/superpowers/specs/2026-05-20-phase-1-ui-design.md) and [implementation plan](docs/superpowers/plans/2026-05-20-phase-1-ui.md).

### Phase 2 (SME UI + AI MCP)
```
make neo4j-up
KG_SCHEMA=$(pwd)/kg-schema.yaml NEO4J_PASSWORD=testtest cargo run -p kg-server
cd webui && npm install && npm run dev
# AI side, separate terminal:
KG_SERVER_URL=http://localhost:9000 cargo run -p kg-mcp
```

See:
- [Phase 2 design](docs/superpowers/specs/2026-05-20-phase-2-sme-ai-design.md)
- [Phase 2 plan](docs/superpowers/plans/2026-05-20-phase-2-sme-ai.md)
- [Manual UI smoke](docs/manual-ui-smoke.md)
- [Manual MCP smoke](docs/manual-mcp-smoke.md)

## Docs

`cargo doc --no-deps --all-features --open`

See:
- [Design spec](docs/superpowers/specs/2026-05-19-graph-editor-framework-design.md)
- [Implementation plan](docs/superpowers/plans/2026-05-19-kg-editor-framework.md)
- [Manual WASM smoke](docs/manual-smoke.md)

## License

MIT OR Apache-2.0
