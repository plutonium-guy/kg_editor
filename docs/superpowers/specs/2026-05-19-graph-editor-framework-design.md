# Phase 0 — Knowledge Graph Editor Framework (Rust + WASM)

**Date:** 2026-05-19
**Status:** Design — pending user approval
**Owner:** amiyamandal

## 1. Purpose

A Rust library (with WASM bindings) for performing CRUD operations and link
management against Neo4j. Phase 0 ships a pure-Rust crate that backend Rust
services can embed directly, plus a `wasm32-unknown-unknown` build for
browser-side consumers that reach Neo4j via the HTTP Query API. The framework
exposes an editor-style staging API (Unit of Work) so multi-step graph edits
commit as a single Cypher transaction.

This phase intentionally excludes: a bundled HTTP server, support for graph
databases other than Neo4j, SSO/Kerberos/bearer auth, GraphQL/REST adapters,
and any UI. Those belong to later phases.

## 2. Goals & Non-goals

### Goals
- Idiomatic Rust API for node, relationship, schema/constraint, and bulk graph
  CRUD against Neo4j 5.x.
- Editor-style Unit of Work that stages mutations and commits them as one
  parameterized Cypher transaction.
- Optional schema registry for opt-in validation and constraint/index
  materialization.
- Single source-truth code that compiles to both native (Bolt transport via
  `neo4rs`) and `wasm32-unknown-unknown` (HTTP Query API via `fetch`).
- Strong unit-test coverage of pure logic (Cypher emission, validation,
  ordering) without requiring Neo4j to be running.

### Non-goals
- Other graph databases (Memgraph, ArangoDB, AGE) — deferred.
- A `GraphStore` trait abstracting over backends — deferred.
- Server binary, REST/GraphQL gateway, websocket sync — deferred.
- Auth beyond username/password (no SSO, no bearer, no Kerberos).
- Connection-level retry/backoff policies beyond what `neo4rs` provides.
- Custom query planner or Cypher parser — we emit Cypher, we do not parse it.

## 3. High-Level Architecture

Workspace with two crates:

```
kg_editor/
├── Cargo.toml                   # workspace manifest
├── crates/
│   ├── kg-core/                 # pure, no I/O, wasm-trivial
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── value.rs         # PropValue
│   │   │   ├── node.rs          # Node, NodeId, LocalId, NodeRef
│   │   │   ├── rel.rs           # Rel, RelType
│   │   │   ├── schema/          # SchemaRegistry, NodeSchema, RelSchema
│   │   │   ├── uow/             # UnitOfWork, StagedOp, CommitPlan
│   │   │   ├── cypher/          # CypherEmitter
│   │   │   └── error.rs         # CoreError
│   │   └── Cargo.toml
│   └── kg-neo4j/                # transports + facade
│       ├── src/
│       │   ├── lib.rs           # Client, ClientBuilder
│       │   ├── transport/
│       │   │   ├── mod.rs       # Transport trait (crate-private)
│       │   │   ├── bolt.rs      # #[cfg(feature="native")]
│       │   │   └── http.rs      # #[cfg(feature="wasm")]
│       │   ├── convert.rs       # PropValue <-> BoltType / JSON
│       │   ├── auth.rs          # Basic auth
│       │   └── error.rs         # Neo4jError, TransportError
│       └── Cargo.toml
└── examples/
    ├── native_crud.rs
    └── wasm_crud/               # wasm-pack target
```

Boundary contract: `kg-core` knows nothing about HTTP, Bolt, tokio, or
`wasm-bindgen`. All I/O lives in `kg-neo4j::transport`. The `Transport` trait
is crate-private — a `Client` is concrete; the feature flag selects the
transport at compile time.

## 4. Consumer Flow

```rust
use kg_neo4j::{ClientBuilder, auth::basic};
use kg_core::{schema::{SchemaRegistry, NodeSchema, RelSchema, PropType}, uow::UnitOfWork, value::PropValue};

let mut reg = SchemaRegistry::new();
reg.add_node(
    NodeSchema::builder("Person")
        .prop("name", PropType::String).required()
        .unique(["name"])
        .build(),
);
reg.add_rel(RelSchema::builder("KNOWS").endpoints("Person", "Person").build());

let client = ClientBuilder::new("bolt://localhost:7687")
    .auth(basic("neo4j", "test"))
    .schema(reg)
    .build()
    .await?;

client.materialize_schema().await?;        // CREATE CONSTRAINT / INDEX

let mut uow = client.unit_of_work();
let alice = uow.create_node(["Person"], [("name", PropValue::from("Alice"))]);
let bob   = uow.create_node(["Person"], [("name", PropValue::from("Bob"))]);
uow.create_rel(alice, bob, "KNOWS", []);

let result = client.commit(uow).await?;
let alice_id = result.id_map[&alice];
```

## 5. Core Data Types

```rust
// kg-core/src/value.rs
pub enum PropValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<PropValue>),
    Map(BTreeMap<String, PropValue>),
    Date(chrono::NaiveDate),
    DateTime(chrono::DateTime<chrono::FixedOffset>),
    Duration(chrono::Duration),
    Point2D { srid: i32, x: f64, y: f64 },
}

// kg-core/src/node.rs
pub struct NodeId(pub i64);
pub struct LocalId(pub u64);
pub enum NodeRef { Server(NodeId), Local(LocalId) }
pub struct Node {
    pub id: Option<NodeId>,
    pub labels: smallvec::SmallVec<[String; 2]>,
    pub props: BTreeMap<String, PropValue>,
}

// kg-core/src/rel.rs
pub struct RelId(pub i64);
pub struct Rel {
    pub id: Option<RelId>,
    pub r#type: String,
    pub start: NodeRef,
    pub end:   NodeRef,
    pub props: BTreeMap<String, PropValue>,
}
```

The `PropValue` variants are aligned to Neo4j's Bolt type system so the
conversion layer in `kg-neo4j::convert` is total.

## 6. Unit of Work

`UnitOfWork` accumulates staged operations. Operations supported:

- `create_node(labels, props) -> LocalId`
- `merge_node(labels, key_props, set_props) -> LocalId` (idempotent)
- `update_node(NodeId, patch: PropPatch)`
- `delete_node(NodeId, cascade: CascadeRule)`
- `create_rel(start: NodeRef, end: NodeRef, type, props) -> LocalId`
- `merge_rel(start, end, type, key_props, set_props) -> LocalId`
- `update_rel(RelId, patch: PropPatch)`
- `delete_rel(RelId)`
- `ensure_constraint(NodeConstraint)` / `ensure_index(IndexSpec)`

`PropPatch` is a sparse map `BTreeMap<String, Option<PropValue>>` —
`Some` sets, `None` removes the property.

`CascadeRule` is `Detach` (DETACH DELETE) or `Strict` (fail if rels present).

`uow.validate()` runs schema checks if a registry is attached; returns
`Vec<SchemaViolation>` collecting all violations (not fail-fast).
`uow.plan() -> CommitPlan` performs a topological sort of staged ops and
returns the ordered Cypher batch with parameters. `client.commit(uow)` calls
`plan()` then ships the batch in a single Neo4j transaction.

### Ordering rules

The emitter orders ops as:

1. Constraint / index DDL (idempotent).
2. Node creates / merges (LocalId-assigning ops).
3. Rel creates / merges.
4. Node updates.
5. Rel updates.
6. Rel deletes.
7. Node deletes.

A single Cypher statement is emitted per batch, chaining `WITH` and binding
each created entity to a variable (`n_1`, `n_2`, `r_1`, …). Rels referencing
`LocalId` are resolved in-statement; rels referencing `NodeId` use `MATCH`.

### Commit result

```rust
pub struct CommitResult {
    pub id_map: HashMap<LocalId, i64>,    // local -> server id (node or rel)
    pub counters: Counters,                // nodes_created, rels_created, ...
}
```

On error the entire transaction rolls back, `Err(Neo4jError::Tx { .. })` is
returned, and the `UnitOfWork` is left intact for inspection/retry.

## 7. Schema Registry

```rust
pub struct SchemaRegistry {
    nodes: HashMap<String, NodeSchema>,
    rels:  HashMap<String, RelSchema>,
}

pub struct NodeSchema {
    pub label: String,
    pub props: Vec<PropSpec>,
    pub uniqueness: Vec<Vec<String>>,   // composite-unique prop sets
    pub indexes: Vec<Vec<String>>,
}

pub struct RelSchema {
    pub r#type: String,
    pub allowed_endpoints: Vec<(String, String)>,  // (start label, end label)
    pub props: Vec<PropSpec>,
    pub cardinality: Cardinality,
}

pub enum Cardinality { OneToOne, OneToMany, ManyToMany }

pub struct PropSpec {
    pub name: String,
    pub ty: PropType,
    pub required: bool,
    pub default: Option<PropValue>,
}

pub enum PropType {
    Bool, Int, Float, String, Bytes,
    Date, DateTime, Duration, Point,
    List(Box<PropType>), Map,
}
```

Validation surface:
- Required props present on `create_node` / `create_rel`.
- Property types match `PropSpec::ty`.
- Rel endpoint labels in `allowed_endpoints` (when start/end labels known
  inside the UoW; server-only refs are validated as best-effort post-fetch
  or skipped).
- Composite uniqueness is enforced server-side via `CREATE CONSTRAINT`;
  the registry materializes the DDL.

`registry.materialize(&client)` emits idempotent `CREATE CONSTRAINT IF NOT
EXISTS` / `CREATE INDEX IF NOT EXISTS` statements.

## 8. Transport

Crate-private trait:

```rust
pub(crate) trait Transport {
    async fn run_tx(&self, stmts: &[Statement]) -> Result<TxOutcome, TransportError>;
    async fn run_autocommit(&self, stmt: &Statement) -> Result<StatementResult, TransportError>;
}

pub(crate) struct Statement {
    pub cypher: String,
    pub params: BTreeMap<String, PropValue>,
}
```

### Native (`feature = "native"`)

- Wraps `neo4rs::Graph` with internal pool.
- `run_tx` opens an explicit transaction, runs all statements, commits on
  success or rolls back on any error.
- `PropValue` ↔ `neo4rs::BoltType` conversion in `kg-neo4j::convert`.
- Async runtime: tokio (no runtime-agnostic abstraction).

### WASM (`feature = "wasm"`)

- Target: `wasm32-unknown-unknown`, `wasm-bindgen-futures` for await.
- Uses `web-sys::fetch` against Neo4j HTTP Query API v2:
  - Single-tx batch: `POST /db/{db}/query/v2`
  - Multi-step: `POST /db/{db}/query/v2/tx`
- Auth: `Authorization: Basic base64(user:pass)` header.
- Request/response shape: Neo4j HTTP Query API v2 JSON; `PropValue`
  (de)serializes via a custom `serde` impl matching Neo4j's typed JSON
  encoding (`{"$type":"Integer","_value":"42"}` etc).
- No connection pool — `fetch` per call.

### Feature flag wiring

```toml
[features]
default = ["native"]
native  = ["dep:neo4rs", "dep:tokio"]
wasm    = ["dep:wasm-bindgen", "dep:wasm-bindgen-futures",
           "dep:web-sys", "dep:js-sys", "dep:serde-wasm-bindgen",
           "dep:serde_json", "dep:base64"]
```

A `compile_error!` guard rejects builds that enable both `native` and `wasm`
simultaneously.

## 9. Client Facade

```rust
pub struct Client { /* transport + optional registry */ }

impl Client {
    pub fn unit_of_work(&self) -> UnitOfWork;
    pub async fn commit(&self, uow: UnitOfWork) -> Result<CommitResult, Neo4jError>;
    pub async fn commit_unchecked(&self, uow: UnitOfWork) -> Result<CommitResult, Neo4jError>;
    pub async fn fetch_node(&self, id: NodeId) -> Result<Option<Node>, Neo4jError>;
    pub async fn fetch_rel(&self, id: RelId)   -> Result<Option<Rel>, Neo4jError>;
    pub async fn query<T: FromRow>(&self, cypher: &str, params: ParamMap) -> Result<Vec<T>, Neo4jError>;
    pub async fn materialize_schema(&self) -> Result<(), Neo4jError>;
}

pub struct ClientBuilder { /* ... */ }
impl ClientBuilder {
    pub fn new(uri: impl Into<String>) -> Self;
    pub fn auth(self, auth: Auth) -> Self;
    pub fn schema(self, reg: SchemaRegistry) -> Self;
    pub fn database(self, name: impl Into<String>) -> Self;
    pub async fn build(self) -> Result<Client, Neo4jError>;
}
```

`query()` is the escape hatch for arbitrary Cypher reads not covered by typed
CRUD. `FromRow` is a small derive-able trait that maps a Cypher row to a
user struct.

## 10. Error Model

```rust
// kg-core
pub enum CoreError {
    SchemaViolation(Vec<SchemaViolation>),
    UnresolvedLocalId(LocalId),
    InvalidPatch { field: String, reason: String },
    CycleInPlan,
}

// kg-neo4j
pub enum Neo4jError {
    Core(CoreError),
    Tx { code: String, message: String },          // server-side Cypher error
    Transport(TransportError),
    Auth(String),
    Conversion { from: &'static str, to: &'static str, reason: String },
}

pub enum TransportError {
    Connect(String),
    Timeout,
    Protocol(String),                                // Bolt-side
    Http { status: u16, body: String },              // wasm-side
}
```

All public errors implement `std::error::Error`. `Send + Sync` bounds apply on
native; on wasm32 they're conditional.

## 11. Testing Strategy

### Tier 1 — `kg-core` unit tests (no Neo4j, no Docker)
- `PropValue` (de)serialization round-trips for every variant.
- `CypherEmitter` snapshot tests via `insta` for canonical UoW shapes:
  pure creates, mixed create+rel, patch update, cascade delete, constraint
  materialization, LocalId resolution.
- Schema validation: missing required prop, wrong type, disallowed
  endpoint, missing label.
- Topological sort: cycles produce `CoreError::CycleInPlan`.

### Tier 2 — `kg-neo4j` integration tests (Docker)
- `tests/` directory, gated `#[cfg(feature="native")]`.
- `testcontainers-rs` spins up `neo4j:5-community` per test module.
- Real CRUD, real transactions, real constraint create/drop, rollback on
  injected errors.
- Conversion round-trips (`PropValue` -> `BoltType` -> server -> `BoltType`
  -> `PropValue`).

### Tier 3 — WASM smoke tests
- `wasm-bindgen-test` against an injectable HTTP client. The `http.rs`
  transport exposes a `HttpClient` trait internally with two impls: the
  real `web-sys::fetch`-backed one for production builds, and a recording
  in-memory impl used only in tests.
- Assert request URL, method, headers, and body shape match the Neo4j
  HTTP Query API v2 spec for each statement type.
- No live browser-to-Neo4j run in CI; one manual end-to-end smoke is
  required before phase-0 sign-off and recorded in `docs/manual-smoke.md`.

## 12. Dev Workflow

```
make neo4j-up           # docker run -d -p7687:7687 -p7474:7474 \
                        #   -e NEO4J_AUTH=neo4j/test neo4j:5-community
cargo test -p kg-core
cargo test -p kg-neo4j --features native
wasm-pack test --headless --chrome -p kg-neo4j --features wasm
cargo doc --no-deps --all-features
```

## 13. CI

- GitHub Actions matrix:
  - native: ubuntu-latest + macos-latest, Neo4j service container.
  - wasm:   ubuntu-latest, headless Chrome.
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo fmt -- --check`
- Coverage report via `cargo llvm-cov`; fail under 80% on `kg-core`.

## 14. Phase-0 Acceptance Criteria

1. Native build: every CRUD operation listed in §6 works against
   `neo4j:5-community` via Docker. A UoW that mixes node creates, rel
   creates referencing `LocalId`s, updates, and deletes commits in a single
   Cypher transaction. Rollback on any error.
2. Schema registry: declared constraints and indexes materialize idempotently
   via `materialize_schema()`. Validation catches missing required props,
   wrong types, and disallowed rel endpoints before commit.
3. WASM build: same public API surface compiles for `wasm32-unknown-unknown`.
   One recorded manual end-to-end run shows browser-side commit succeeding
   against Neo4j HTTP Query API v2.
4. Tests: `kg-core` ≥ 80% line coverage. All three test tiers green in CI.
5. Docs: every public type and method has a doc comment with at least one
   example for the top-level entry points.

## 15. Out of Scope / Phase 1+

- Other graph databases (Memgraph, ArangoDB, AGE).
- Real-time subscriptions / change-data-capture.
- Authentication beyond basic (SSO, bearer, Kerberos).
- Bundled HTTP/gRPC server.
- TypeScript bindings beyond raw wasm-bindgen output.
- Visual editor UI.
- Migration tooling (schema diff / evolution).

## 16. Open Questions

None blocking. The following will be revisited during plan writing if they
become decisions:

- Pool sizing defaults for `neo4rs` on native.
- Whether `query()` should support streaming results or buffered only (the
  spec assumes buffered).
- Public-facing crate naming on crates.io (`kg-core`, `kg-neo4j` proposed).
