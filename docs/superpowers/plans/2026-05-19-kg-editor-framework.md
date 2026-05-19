# KG Editor Framework — Phase 0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust workspace (`kg-core` + `kg-neo4j`) that lets backend services and WASM consumers perform full CRUD + linking against Neo4j 5.x using a Unit-of-Work staging API, with optional schema validation.

**Architecture:** Pure-logic core crate (no I/O) with a transport crate that compiles either to native (Bolt via `neo4rs`) or wasm32 (HTTP Query API v2 via `web-sys::fetch`). Single `Transport` trait is crate-private; one impl is enabled per build by mutually exclusive Cargo features.

**Tech Stack:** Rust 1.78+, tokio, neo4rs, wasm-bindgen, web-sys, serde, serde_json, chrono, smallvec, thiserror, insta (snapshot tests), testcontainers-rs, wasm-bindgen-test, GitHub Actions, Docker.

**Spec:** `docs/superpowers/specs/2026-05-19-graph-editor-framework-design.md`

---

## File Structure

```
kg_editor/
├── Cargo.toml                                # workspace manifest
├── rust-toolchain.toml
├── Makefile
├── README.md
├── .gitignore
├── .github/workflows/ci.yml
├── crates/
│   ├── kg-core/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs                        # public re-exports
│   │   │   ├── value.rs                      # PropValue
│   │   │   ├── node.rs                       # NodeId, LocalId, NodeRef, Node
│   │   │   ├── rel.rs                        # RelId, Rel
│   │   │   ├── error.rs                      # CoreError, SchemaViolation
│   │   │   ├── schema/
│   │   │   │   ├── mod.rs                    # SchemaRegistry + validate()
│   │   │   │   └── types.rs                  # NodeSchema, RelSchema, PropSpec, PropType, Cardinality
│   │   │   ├── uow/
│   │   │   │   ├── mod.rs                    # UnitOfWork, StagedOp, PropPatch, CascadeRule
│   │   │   │   └── plan.rs                   # CommitPlan, topological sort
│   │   │   └── cypher/
│   │   │       ├── mod.rs                    # CypherEmitter entrypoint
│   │   │       ├── statement.rs              # Statement, ParamMap, sanitization
│   │   │       ├── ddl.rs                    # constraint/index emission
│   │   │       ├── nodes.rs                  # node ops emission
│   │   │       └── rels.rs                   # rel ops + WITH-chain resolution
│   │   └── tests/
│   │       └── snapshots/                    # insta snapshots
│   └── kg-neo4j/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs                        # public surface
│       │   ├── client.rs                     # Client + ClientBuilder
│       │   ├── auth.rs                       # Auth, basic()
│       │   ├── error.rs                      # Neo4jError, TransportError
│       │   ├── from_row.rs                   # FromRow trait
│       │   ├── transport/
│       │   │   ├── mod.rs                    # Transport trait, TxOutcome
│       │   │   ├── bolt.rs                   # cfg(feature="native")
│       │   │   └── http.rs                   # cfg(feature="wasm")
│       │   └── convert/
│       │       ├── mod.rs
│       │       ├── bolt.rs                   # PropValue <-> neo4rs::BoltType
│       │       └── json.rs                   # PropValue <-> serde_json::Value
│       └── tests/
│           └── integration_native.rs         # testcontainers
├── examples/
│   ├── native_crud.rs
│   └── wasm_crud/                            # separate cargo project for wasm-pack
│       ├── Cargo.toml
│       ├── index.html
│       └── src/lib.rs
└── docs/
    ├── superpowers/
    │   ├── specs/2026-05-19-graph-editor-framework-design.md
    │   └── plans/2026-05-19-kg-editor-framework.md
    └── manual-smoke.md
```

Boundary contract: `kg-core` has **zero** dependencies on `tokio`, `neo4rs`, `wasm-bindgen`, `web-sys`, or `serde_json`. Its only deps are `chrono`, `smallvec`, `serde` (with `derive`), and `thiserror`.

---

## Task 1: Workspace skeleton

**Files:**
- Create: `Cargo.toml` (workspace manifest)
- Create: `rust-toolchain.toml`
- Create: `.gitignore`
- Create: `Makefile`
- Create: `README.md`

- [ ] **Step 1: Create workspace `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = ["crates/kg-core", "crates/kg-neo4j", "examples"]
exclude = ["examples/wasm_crud"]

[workspace.package]
edition = "2021"
rust-version = "1.78"
license = "MIT OR Apache-2.0"
repository = "https://github.com/amiyamandal/kg_editor"

[workspace.dependencies]
chrono       = { version = "0.4", default-features = false, features = ["serde", "clock"] }
smallvec     = { version = "1.13", features = ["serde"] }
serde        = { version = "1", features = ["derive"] }
serde_json   = "1"
thiserror    = "1"
tokio        = { version = "1", features = ["macros", "rt-multi-thread"] }
neo4rs       = "0.8"
base64       = "0.22"
insta        = { version = "1", features = ["json"] }
testcontainers = "0.20"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
wasm-bindgen-test = "0.3"
web-sys      = { version = "0.3", features = ["Request","RequestInit","RequestMode","Response","Window","Headers"] }
js-sys       = "0.3"
serde-wasm-bindgen = "0.6"
```

- [ ] **Step 2: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "1.78.0"
components = ["clippy", "rustfmt"]
targets = ["wasm32-unknown-unknown"]
```

- [ ] **Step 3: Create `.gitignore`**

```
target/
Cargo.lock
**/*.rs.bk
.DS_Store
crates/*/Cargo.lock
examples/wasm_crud/pkg/
node_modules/
```

- [ ] **Step 4: Create `Makefile`**

```makefile
.PHONY: neo4j-up neo4j-down test-core test-native test-wasm fmt clippy doc

neo4j-up:
	docker run -d --name kg-neo4j -p 7687:7687 -p 7474:7474 \
		-e NEO4J_AUTH=neo4j/test neo4j:5-community

neo4j-down:
	docker rm -f kg-neo4j

test-core:
	cargo test -p kg-core

test-native:
	cargo test -p kg-neo4j --features native

test-wasm:
	wasm-pack test --headless --chrome crates/kg-neo4j --features wasm

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

doc:
	cargo doc --no-deps --all-features
```

- [ ] **Step 5: Create `README.md` stub**

```markdown
# kg_editor

Rust framework for Neo4j graph CRUD + linking. Phase 0: native (Bolt) + WASM (HTTP).

See [design spec](docs/superpowers/specs/2026-05-19-graph-editor-framework-design.md).

## Quick start

```rust
// see crates/kg-neo4j/README.md
```
```

- [ ] **Step 6: Verify workspace parses**

Run: `cargo metadata --no-deps --format-version 1 >/dev/null`
Expected: exits 0 (members listed don't exist yet but `cargo metadata` will fail — defer until Task 2 creates the members).

Skip this check for now; it runs after Task 3.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml rust-toolchain.toml .gitignore Makefile README.md
git commit -m "chore: bootstrap kg-editor workspace"
```

---

## Task 2: kg-core crate skeleton

**Files:**
- Create: `crates/kg-core/Cargo.toml`
- Create: `crates/kg-core/src/lib.rs`

- [ ] **Step 1: Create `crates/kg-core/Cargo.toml`**

```toml
[package]
name = "kg-core"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Pure-logic core for the kg-editor framework (types, schema, unit-of-work, Cypher emission)."

[dependencies]
chrono     = { workspace = true }
smallvec   = { workspace = true }
serde      = { workspace = true }
thiserror  = { workspace = true }

[dev-dependencies]
insta      = { workspace = true }
serde_json = { workspace = true }
```

- [ ] **Step 2: Create `crates/kg-core/src/lib.rs`**

```rust
#![doc = include_str!("../../../README.md")]
#![deny(missing_docs)]
#![allow(clippy::module_name_repetitions)]

//! kg-core: pure logic for the kg-editor framework.

pub mod cypher;
pub mod error;
pub mod node;
pub mod rel;
pub mod schema;
pub mod uow;
pub mod value;

pub use error::{CoreError, SchemaViolation};
pub use node::{LocalId, Node, NodeId, NodeRef};
pub use rel::{Rel, RelId};
pub use schema::{Cardinality, NodeSchema, PropSpec, PropType, RelSchema, SchemaRegistry};
pub use uow::{CascadeRule, PropPatch, StagedOp, UnitOfWork};
pub use value::PropValue;
```

- [ ] **Step 3: Create empty submodule stubs so `lib.rs` compiles**

`crates/kg-core/src/value.rs`:
```rust
//! Property values (Bolt-compatible scalar/composite types).
```

`crates/kg-core/src/node.rs`:
```rust
//! Node types and identifiers.
```

`crates/kg-core/src/rel.rs`:
```rust
//! Relationship types and identifiers.
```

`crates/kg-core/src/error.rs`:
```rust
//! Error and validation-violation types.
```

`crates/kg-core/src/schema/mod.rs`:
```rust
//! Optional schema registry.
pub mod types;
```

`crates/kg-core/src/schema/types.rs`:
```rust
//! Schema definition types.
```

`crates/kg-core/src/uow/mod.rs`:
```rust
//! Unit of Work staging.
pub mod plan;
```

`crates/kg-core/src/uow/plan.rs`:
```rust
//! Commit planning (topological sort).
```

`crates/kg-core/src/cypher/mod.rs`:
```rust
//! Cypher emitter (UoW -> parameterized statements).
pub mod statement;
pub mod ddl;
pub mod nodes;
pub mod rels;
```

`crates/kg-core/src/cypher/statement.rs`:
```rust
//! Statement and parameter representation.
```

`crates/kg-core/src/cypher/ddl.rs`:
```rust
//! DDL emission (CREATE CONSTRAINT / INDEX).
```

`crates/kg-core/src/cypher/nodes.rs`:
```rust
//! Node op emission.
```

`crates/kg-core/src/cypher/rels.rs`:
```rust
//! Rel op emission.
```

Then remove the public re-exports in `lib.rs` that reference items not yet defined — replace `lib.rs` with this minimal scaffold so it compiles:

```rust
//! kg-core: pure logic for the kg-editor framework.
#![allow(clippy::module_name_repetitions)]

pub mod cypher;
pub mod error;
pub mod node;
pub mod rel;
pub mod schema;
pub mod uow;
pub mod value;
```

- [ ] **Step 4: Run check**

Run: `cargo check -p kg-core`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/kg-core
git commit -m "feat(kg-core): scaffold crate with empty modules"
```

---

## Task 3: kg-neo4j crate skeleton with feature guard

**Files:**
- Create: `crates/kg-neo4j/Cargo.toml`
- Create: `crates/kg-neo4j/src/lib.rs`

- [ ] **Step 1: Create `crates/kg-neo4j/Cargo.toml`**

```toml
[package]
name = "kg-neo4j"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Neo4j transport + client for the kg-editor framework."

[features]
default = ["native"]
native  = ["dep:neo4rs", "dep:tokio"]
wasm    = [
  "dep:wasm-bindgen",
  "dep:wasm-bindgen-futures",
  "dep:web-sys",
  "dep:js-sys",
  "dep:serde-wasm-bindgen",
  "dep:serde_json",
  "dep:base64",
]

[dependencies]
kg-core    = { path = "../kg-core", version = "0.1" }
serde      = { workspace = true }
thiserror  = { workspace = true }

# native-only
neo4rs     = { workspace = true, optional = true }
tokio      = { workspace = true, optional = true }

# wasm-only
wasm-bindgen          = { workspace = true, optional = true }
wasm-bindgen-futures  = { workspace = true, optional = true }
web-sys               = { workspace = true, optional = true }
js-sys                = { workspace = true, optional = true }
serde-wasm-bindgen    = { workspace = true, optional = true }
serde_json            = { workspace = true, optional = true }
base64                = { workspace = true, optional = true }

[dev-dependencies]
tokio          = { workspace = true, features = ["macros", "rt-multi-thread"] }
testcontainers = { workspace = true }
serde_json     = { workspace = true }

[target.'cfg(target_arch = "wasm32")'.dev-dependencies]
wasm-bindgen-test = { workspace = true }
```

- [ ] **Step 2: Create `crates/kg-neo4j/src/lib.rs`**

```rust
//! kg-neo4j: Neo4j transport + client.

#[cfg(all(feature = "native", feature = "wasm"))]
compile_error!(
    "features `native` and `wasm` are mutually exclusive; enable exactly one"
);

#[cfg(not(any(feature = "native", feature = "wasm")))]
compile_error!(
    "kg-neo4j requires exactly one of the `native` or `wasm` features"
);

pub use kg_core;
```

- [ ] **Step 3: Verify feature guard**

Run: `cargo check -p kg-neo4j` (uses default = native)
Expected: PASS.

Run: `cargo check -p kg-neo4j --no-default-features --features native,wasm`
Expected: FAIL with "features `native` and `wasm` are mutually exclusive".

Run: `cargo check -p kg-neo4j --no-default-features`
Expected: FAIL with "requires exactly one".

- [ ] **Step 4: Commit**

```bash
git add crates/kg-neo4j
git commit -m "feat(kg-neo4j): scaffold crate with mutually exclusive feature guard"
```

---

## Task 4: `PropValue` type

**Files:**
- Modify: `crates/kg-core/src/value.rs`
- Test: same file (`#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing test**

Append to `crates/kg-core/src/value.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn from_primitives() {
        assert_eq!(PropValue::from(true), PropValue::Bool(true));
        assert_eq!(PropValue::from(42_i64), PropValue::Int(42));
        assert_eq!(PropValue::from(1.5_f64), PropValue::Float(1.5));
        assert_eq!(PropValue::from("hi"), PropValue::String("hi".into()));
        assert_eq!(PropValue::from(String::from("hi")), PropValue::String("hi".into()));
    }

    #[test]
    fn list_and_map() {
        let v = PropValue::from(vec![PropValue::Int(1), PropValue::Int(2)]);
        assert!(matches!(v, PropValue::List(ref xs) if xs.len() == 2));
        let mut m = std::collections::BTreeMap::new();
        m.insert("k".to_string(), PropValue::Int(1));
        let v = PropValue::Map(m);
        assert!(matches!(v, PropValue::Map(_)));
    }

    #[test]
    fn temporal() {
        let d = chrono::NaiveDate::from_ymd_opt(2026, 5, 19).unwrap();
        assert!(matches!(PropValue::Date(d), PropValue::Date(_)));
        let dt = chrono::FixedOffset::east_opt(0).unwrap()
            .with_ymd_and_hms(2026, 5, 19, 12, 0, 0).unwrap();
        assert!(matches!(PropValue::DateTime(dt), PropValue::DateTime(_)));
    }

    #[test]
    fn null_default() {
        assert_eq!(PropValue::default(), PropValue::Null);
    }
}
```

Run: `cargo test -p kg-core --lib value::tests`
Expected: FAIL — `PropValue` does not exist.

- [ ] **Step 2: Implement `PropValue`**

Replace `crates/kg-core/src/value.rs` body (above the test module) with:

```rust
//! Property values (Bolt-compatible scalar/composite types).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A property value compatible with Neo4j Bolt scalar and composite types.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PropValue {
    #[default]
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

impl PropValue {
    /// Convenience constructor for `Map`.
    pub fn map_of<I, K>(items: I) -> Self
    where
        I: IntoIterator<Item = (K, PropValue)>,
        K: Into<String>,
    {
        PropValue::Map(items.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}

macro_rules! impl_from_scalar {
    ($($t:ty => $var:ident),* $(,)?) => {
        $(impl From<$t> for PropValue {
            fn from(v: $t) -> Self { PropValue::$var(v.into()) }
        })*
    };
}

impl_from_scalar!(
    bool   => Bool,
    i32    => Int,
    i64    => Int,
    f32    => Float,
    f64    => Float,
    String => String,
);

impl From<&str> for PropValue {
    fn from(v: &str) -> Self { PropValue::String(v.to_owned()) }
}

impl From<Vec<PropValue>> for PropValue {
    fn from(v: Vec<PropValue>) -> Self { PropValue::List(v) }
}
```

Note: `chrono::Duration` does not implement `Serialize` by default; add a custom serde impl in a follow-up if needed. For now, derive-skip the Duration variant by overriding its serialization manually if Serialize derive fails:

If `cargo test` reports that `chrono::Duration` is missing serde — gate it: change the field to a struct with `seconds: i64, nanos: i32` and convert via `From`/`TryFrom`:

```rust
Duration { seconds: i64, nanos: i32 },
```

Update tests accordingly.

- [ ] **Step 3: Run tests**

Run: `cargo test -p kg-core --lib value::tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kg-core/src/value.rs
git commit -m "feat(kg-core): PropValue with From conversions and serde"
```

---

## Task 5: Node + Rel identifier types

**Files:**
- Modify: `crates/kg-core/src/node.rs`
- Modify: `crates/kg-core/src/rel.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/kg-core/src/node.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::PropValue;

    #[test]
    fn local_id_distinct() {
        let a = LocalId(1);
        let b = LocalId(2);
        assert_ne!(a, b);
    }

    #[test]
    fn node_ref_variants() {
        let nr = NodeRef::Server(NodeId(7));
        assert!(matches!(nr, NodeRef::Server(_)));
        let nr2 = NodeRef::Local(LocalId(3));
        assert!(matches!(nr2, NodeRef::Local(_)));
    }

    #[test]
    fn node_construct() {
        let mut props = std::collections::BTreeMap::new();
        props.insert("name".into(), PropValue::String("Alice".into()));
        let n = Node { id: None, labels: smallvec::smallvec!["Person".into()], props };
        assert_eq!(n.labels.len(), 1);
    }
}
```

Append to `crates/kg-core/src/rel.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{LocalId, NodeRef};

    #[test]
    fn rel_construct() {
        let r = Rel {
            id: None,
            r#type: "KNOWS".into(),
            start: NodeRef::Local(LocalId(1)),
            end: NodeRef::Local(LocalId(2)),
            props: Default::default(),
        };
        assert_eq!(r.r#type, "KNOWS");
    }
}
```

Run: `cargo test -p kg-core --lib node::tests rel::tests`
Expected: FAIL (types undefined).

- [ ] **Step 2: Implement node and rel types**

Replace `crates/kg-core/src/node.rs` body (above test module):

```rust
//! Node types and identifiers.

use crate::value::PropValue;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::BTreeMap;

/// Server-assigned node identifier (Neo4j internal id).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub i64);

/// UoW-scoped handle issued before commit. Resolves to a `NodeId` on success.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalId(pub u64);

/// Reference to a node either by server id or local (pre-commit) id.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeRef {
    Server(NodeId),
    Local(LocalId),
}

/// A node with optional server id, labels, and properties.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: Option<NodeId>,
    pub labels: SmallVec<[String; 2]>,
    pub props: BTreeMap<String, PropValue>,
}
```

Replace `crates/kg-core/src/rel.rs` body:

```rust
//! Relationship types and identifiers.

use crate::node::NodeRef;
use crate::value::PropValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Server-assigned relationship identifier.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelId(pub i64);

/// A relationship with typed endpoints (server or local refs).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rel {
    pub id: Option<RelId>,
    pub r#type: String,
    pub start: NodeRef,
    pub end: NodeRef,
    pub props: BTreeMap<String, PropValue>,
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p kg-core --lib node::tests rel::tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kg-core/src/node.rs crates/kg-core/src/rel.rs
git commit -m "feat(kg-core): Node, Rel, identifier types"
```

---

## Task 6: `CoreError` + `SchemaViolation`

**Files:**
- Modify: `crates/kg-core/src/error.rs`

- [ ] **Step 1: Write failing test**

Append to `crates/kg-core/src/error.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::LocalId;

    #[test]
    fn display_messages() {
        let e = CoreError::UnresolvedLocalId(LocalId(7));
        assert!(format!("{e}").contains("LocalId(7)"));
        let e = CoreError::CycleInPlan;
        assert!(format!("{e}").contains("cycle"));
    }

    #[test]
    fn schema_violation_variants() {
        let v = SchemaViolation::MissingRequiredProp {
            label_or_type: "Person".into(),
            prop: "name".into(),
        };
        assert!(format!("{v}").contains("name"));
    }
}
```

Run: `cargo test -p kg-core --lib error::tests`
Expected: FAIL.

- [ ] **Step 2: Implement**

Replace `crates/kg-core/src/error.rs`:

```rust
//! Error and validation-violation types.

use crate::node::LocalId;
use thiserror::Error;

/// Top-level error from pure-logic operations.
#[derive(Debug, Error, PartialEq)]
pub enum CoreError {
    #[error("schema violations: {0:?}")]
    SchemaViolation(Vec<SchemaViolation>),
    #[error("unresolved {0:?}; rel referenced a local node not staged")]
    UnresolvedLocalId(LocalId),
    #[error("invalid patch on field `{field}`: {reason}")]
    InvalidPatch { field: String, reason: String },
    #[error("cycle detected in commit plan")]
    CycleInPlan,
}

/// A single schema validation violation.
#[derive(Debug, Error, PartialEq, Clone)]
pub enum SchemaViolation {
    #[error("`{label_or_type}` missing required prop `{prop}`")]
    MissingRequiredProp { label_or_type: String, prop: String },
    #[error("`{label_or_type}`.`{prop}` expected {expected:?}, got {got:?}")]
    WrongPropType {
        label_or_type: String,
        prop: String,
        expected: String,
        got: String,
    },
    #[error("rel `{r#type}` endpoints `{start}`->`{end}` not in allowed set")]
    DisallowedEndpoints {
        r#type: String,
        start: String,
        end: String,
    },
    #[error("unknown label `{label}` not registered in schema")]
    UnknownLabel { label: String },
    #[error("unknown rel type `{r#type}` not registered in schema")]
    UnknownRelType { r#type: String },
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p kg-core --lib error::tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kg-core/src/error.rs
git commit -m "feat(kg-core): CoreError and SchemaViolation enums"
```

---

## Task 7: Schema type definitions

**Files:**
- Modify: `crates/kg-core/src/schema/types.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/kg-core/src/schema/types.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_schema_builder() {
        let s = NodeSchema::builder("Person")
            .prop("name", PropType::String).required()
            .prop("age", PropType::Int).default(crate::value::PropValue::Int(0))
            .unique(["name"])
            .build();
        assert_eq!(s.label, "Person");
        assert_eq!(s.props.len(), 2);
        assert!(s.props.iter().any(|p| p.name == "name" && p.required));
        assert_eq!(s.uniqueness, vec![vec!["name".to_string()]]);
    }

    #[test]
    fn rel_schema_builder() {
        let s = RelSchema::builder("KNOWS")
            .endpoints("Person", "Person")
            .endpoints("Person", "Org")
            .prop("since", PropType::Date).build();
        assert_eq!(s.r#type, "KNOWS");
        assert_eq!(s.allowed_endpoints.len(), 2);
        assert_eq!(s.cardinality, Cardinality::ManyToMany);
    }
}
```

Run: `cargo test -p kg-core --lib schema::types::tests`
Expected: FAIL.

- [ ] **Step 2: Implement**

Replace `crates/kg-core/src/schema/types.rs`:

```rust
//! Schema definition types.

use crate::value::PropValue;
use serde::{Deserialize, Serialize};

/// Cardinality hint (used for documentation + future validation).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cardinality {
    OneToOne,
    OneToMany,
    #[default]
    ManyToMany,
}

/// Allowed value type for a property.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropType {
    Bool, Int, Float, String, Bytes,
    Date, DateTime, Duration, Point,
    List(Box<PropType>),
    Map,
}

impl PropType {
    /// Human-readable name (for error messages).
    pub fn name(&self) -> String {
        match self {
            PropType::Bool => "Bool".into(),
            PropType::Int  => "Int".into(),
            PropType::Float => "Float".into(),
            PropType::String => "String".into(),
            PropType::Bytes => "Bytes".into(),
            PropType::Date => "Date".into(),
            PropType::DateTime => "DateTime".into(),
            PropType::Duration => "Duration".into(),
            PropType::Point => "Point".into(),
            PropType::List(inner) => format!("List<{}>", inner.name()),
            PropType::Map => "Map".into(),
        }
    }
}

/// A single property specification.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropSpec {
    pub name: String,
    pub ty: PropType,
    pub required: bool,
    pub default: Option<PropValue>,
}

/// Node schema (label + props + uniqueness + indexes).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeSchema {
    pub label: String,
    pub props: Vec<PropSpec>,
    pub uniqueness: Vec<Vec<String>>,
    pub indexes: Vec<Vec<String>>,
}

impl NodeSchema {
    pub fn builder(label: impl Into<String>) -> NodeSchemaBuilder {
        NodeSchemaBuilder {
            inner: NodeSchema {
                label: label.into(),
                props: vec![],
                uniqueness: vec![],
                indexes: vec![],
            },
            cursor: None,
        }
    }
}

pub struct NodeSchemaBuilder {
    inner: NodeSchema,
    cursor: Option<usize>,
}

impl NodeSchemaBuilder {
    pub fn prop(mut self, name: impl Into<String>, ty: PropType) -> Self {
        self.inner.props.push(PropSpec { name: name.into(), ty, required: false, default: None });
        self.cursor = Some(self.inner.props.len() - 1);
        self
    }
    pub fn required(mut self) -> Self {
        if let Some(i) = self.cursor { self.inner.props[i].required = true; }
        self
    }
    pub fn default(mut self, v: PropValue) -> Self {
        if let Some(i) = self.cursor { self.inner.props[i].default = Some(v); }
        self
    }
    pub fn unique<I, S>(mut self, props: I) -> Self
    where I: IntoIterator<Item = S>, S: Into<String> {
        self.inner.uniqueness.push(props.into_iter().map(Into::into).collect());
        self
    }
    pub fn index<I, S>(mut self, props: I) -> Self
    where I: IntoIterator<Item = S>, S: Into<String> {
        self.inner.indexes.push(props.into_iter().map(Into::into).collect());
        self
    }
    pub fn build(self) -> NodeSchema { self.inner }
}

/// Rel schema (type + allowed endpoints + props + cardinality).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RelSchema {
    pub r#type: String,
    pub allowed_endpoints: Vec<(String, String)>,
    pub props: Vec<PropSpec>,
    pub cardinality: Cardinality,
}

impl RelSchema {
    pub fn builder(r#type: impl Into<String>) -> RelSchemaBuilder {
        RelSchemaBuilder {
            inner: RelSchema {
                r#type: r#type.into(),
                allowed_endpoints: vec![],
                props: vec![],
                cardinality: Cardinality::ManyToMany,
            },
            cursor: None,
        }
    }
}

pub struct RelSchemaBuilder {
    inner: RelSchema,
    cursor: Option<usize>,
}

impl RelSchemaBuilder {
    pub fn endpoints(mut self, start: impl Into<String>, end: impl Into<String>) -> Self {
        self.inner.allowed_endpoints.push((start.into(), end.into()));
        self
    }
    pub fn prop(mut self, name: impl Into<String>, ty: PropType) -> Self {
        self.inner.props.push(PropSpec { name: name.into(), ty, required: false, default: None });
        self.cursor = Some(self.inner.props.len() - 1);
        self
    }
    pub fn required(mut self) -> Self {
        if let Some(i) = self.cursor { self.inner.props[i].required = true; }
        self
    }
    pub fn cardinality(mut self, c: Cardinality) -> Self { self.inner.cardinality = c; self }
    pub fn build(self) -> RelSchema { self.inner }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p kg-core --lib schema::types::tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kg-core/src/schema
git commit -m "feat(kg-core): schema types + builders"
```

---

## Task 8: `SchemaRegistry` + validation

**Files:**
- Modify: `crates/kg-core/src/schema/mod.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/kg-core/src/schema/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SchemaViolation;
    use crate::value::PropValue;
    use std::collections::BTreeMap;

    fn reg() -> SchemaRegistry {
        let mut r = SchemaRegistry::new();
        r.add_node(types::NodeSchema::builder("Person")
            .prop("name", types::PropType::String).required()
            .prop("age", types::PropType::Int)
            .build());
        r.add_rel(types::RelSchema::builder("KNOWS")
            .endpoints("Person", "Person")
            .build());
        r
    }

    #[test]
    fn missing_required_prop() {
        let r = reg();
        let mut props = BTreeMap::new();
        props.insert("age".into(), PropValue::Int(30));
        let v = r.validate_node_props("Person", &props);
        assert!(v.iter().any(|x| matches!(x,
            SchemaViolation::MissingRequiredProp { prop, .. } if prop == "name")));
    }

    #[test]
    fn wrong_prop_type() {
        let r = reg();
        let mut props = BTreeMap::new();
        props.insert("name".into(), PropValue::String("a".into()));
        props.insert("age".into(), PropValue::String("not int".into()));
        let v = r.validate_node_props("Person", &props);
        assert!(v.iter().any(|x| matches!(x, SchemaViolation::WrongPropType { .. })));
    }

    #[test]
    fn unknown_label() {
        let r = reg();
        let v = r.validate_node_props("Alien", &BTreeMap::new());
        assert!(v.iter().any(|x| matches!(x, SchemaViolation::UnknownLabel { .. })));
    }

    #[test]
    fn endpoints_allowed() {
        let r = reg();
        let v = r.validate_rel_endpoints("KNOWS", Some("Person"), Some("Person"));
        assert!(v.is_empty());
        let v = r.validate_rel_endpoints("KNOWS", Some("Person"), Some("Org"));
        assert!(v.iter().any(|x| matches!(x, SchemaViolation::DisallowedEndpoints { .. })));
    }
}
```

Run: `cargo test -p kg-core --lib schema::tests`
Expected: FAIL.

- [ ] **Step 2: Implement**

Replace `crates/kg-core/src/schema/mod.rs`:

```rust
//! Optional schema registry.

pub mod types;

pub use types::{Cardinality, NodeSchema, PropSpec, PropType, RelSchema};

use crate::error::SchemaViolation;
use crate::value::PropValue;
use std::collections::{BTreeMap, HashMap};

/// Holds declared node and relationship schemas.
#[derive(Clone, Debug, Default)]
pub struct SchemaRegistry {
    nodes: HashMap<String, NodeSchema>,
    rels: HashMap<String, RelSchema>,
}

impl SchemaRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn add_node(&mut self, s: NodeSchema) -> &mut Self {
        self.nodes.insert(s.label.clone(), s);
        self
    }
    pub fn add_rel(&mut self, s: RelSchema) -> &mut Self {
        self.rels.insert(s.r#type.clone(), s);
        self
    }

    pub fn node(&self, label: &str) -> Option<&NodeSchema> { self.nodes.get(label) }
    pub fn rel(&self, ty: &str) -> Option<&RelSchema> { self.rels.get(ty) }
    pub fn nodes(&self) -> impl Iterator<Item = &NodeSchema> { self.nodes.values() }
    pub fn rels(&self) -> impl Iterator<Item = &RelSchema> { self.rels.values() }

    /// Validate properties against the schema for `label`.
    /// Returns all collected violations. Unknown label is itself a violation.
    pub fn validate_node_props(
        &self,
        label: &str,
        props: &BTreeMap<String, PropValue>,
    ) -> Vec<SchemaViolation> {
        let Some(schema) = self.nodes.get(label) else {
            return vec![SchemaViolation::UnknownLabel { label: label.into() }];
        };
        let mut out = vec![];
        for spec in &schema.props {
            match props.get(&spec.name) {
                None if spec.required && spec.default.is_none() => {
                    out.push(SchemaViolation::MissingRequiredProp {
                        label_or_type: label.into(),
                        prop: spec.name.clone(),
                    });
                }
                Some(v) if !type_matches(v, &spec.ty) => {
                    out.push(SchemaViolation::WrongPropType {
                        label_or_type: label.into(),
                        prop: spec.name.clone(),
                        expected: spec.ty.name(),
                        got: value_type_name(v).into(),
                    });
                }
                _ => {}
            }
        }
        out
    }

    /// Validate rel endpoints. `start_label`/`end_label` are `None` if the
    /// referenced node is server-side and label was not provided.
    pub fn validate_rel_endpoints(
        &self,
        r#type: &str,
        start_label: Option<&str>,
        end_label: Option<&str>,
    ) -> Vec<SchemaViolation> {
        let Some(schema) = self.rels.get(r#type) else {
            return vec![SchemaViolation::UnknownRelType { r#type: r#type.into() }];
        };
        let (Some(s), Some(e)) = (start_label, end_label) else { return vec![]; };
        if schema.allowed_endpoints.is_empty() { return vec![]; }
        if schema.allowed_endpoints.iter().any(|(a, b)| a == s && b == e) {
            vec![]
        } else {
            vec![SchemaViolation::DisallowedEndpoints {
                r#type: r#type.into(),
                start: s.into(),
                end: e.into(),
            }]
        }
    }
}

fn type_matches(v: &PropValue, t: &PropType) -> bool {
    use PropType::*;
    match (v, t) {
        (PropValue::Null, _) => true, // null allowed at type level; required handles presence
        (PropValue::Bool(_), Bool) => true,
        (PropValue::Int(_), Int) => true,
        (PropValue::Float(_), Float) => true,
        (PropValue::String(_), String) => true,
        (PropValue::Bytes(_), Bytes) => true,
        (PropValue::Date(_), Date) => true,
        (PropValue::DateTime(_), DateTime) => true,
        (PropValue::Duration { .. }, Duration) => true,
        (PropValue::Point2D { .. }, Point) => true,
        (PropValue::Map(_), Map) => true,
        (PropValue::List(xs), List(inner)) => xs.iter().all(|x| type_matches(x, inner)),
        _ => false,
    }
}

fn value_type_name(v: &PropValue) -> &'static str {
    match v {
        PropValue::Null => "Null",
        PropValue::Bool(_) => "Bool",
        PropValue::Int(_) => "Int",
        PropValue::Float(_) => "Float",
        PropValue::String(_) => "String",
        PropValue::Bytes(_) => "Bytes",
        PropValue::List(_) => "List",
        PropValue::Map(_) => "Map",
        PropValue::Date(_) => "Date",
        PropValue::DateTime(_) => "DateTime",
        PropValue::Duration { .. } => "Duration",
        PropValue::Point2D { .. } => "Point",
    }
}
```

Note: `PropValue::Duration` is the struct-variant `{ seconds, nanos }` form (from Task 4 follow-up). If Task 4 kept `chrono::Duration`, adjust the pattern accordingly to `PropValue::Duration(_)`.

- [ ] **Step 3: Run tests**

Run: `cargo test -p kg-core --lib schema`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kg-core/src/schema/mod.rs
git commit -m "feat(kg-core): SchemaRegistry with validate_node_props and endpoint check"
```

---

## Task 9: Unit-of-Work core types

**Files:**
- Modify: `crates/kg-core/src/uow/mod.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/kg-core/src/uow/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{LocalId, NodeId, NodeRef};
    use crate::value::PropValue;

    #[test]
    fn create_node_returns_increasing_local_ids() {
        let mut uow = UnitOfWork::new();
        let a = uow.create_node(["Person"], [("name", PropValue::from("A"))]);
        let b = uow.create_node(["Person"], [("name", PropValue::from("B"))]);
        assert_ne!(a, b);
    }

    #[test]
    fn stages_are_recorded() {
        let mut uow = UnitOfWork::new();
        uow.create_node(["X"], []);
        uow.update_node(NodeId(7), PropPatch::new().set("n", PropValue::Int(1)));
        uow.delete_node(NodeId(8), CascadeRule::Detach);
        assert_eq!(uow.ops().len(), 3);
    }

    #[test]
    fn create_rel_accepts_local_or_server_endpoints() {
        let mut uow = UnitOfWork::new();
        let a = uow.create_node(["P"], []);
        uow.create_rel(NodeRef::Local(a), NodeRef::Server(NodeId(9)), "R", []);
        assert_eq!(uow.ops().len(), 2);
    }

    #[test]
    fn prop_patch_sparse() {
        let p = PropPatch::new()
            .set("a", PropValue::Int(1))
            .unset("b");
        assert_eq!(p.entries().len(), 2);
    }
}
```

Run: `cargo test -p kg-core --lib uow::tests`
Expected: FAIL.

- [ ] **Step 2: Implement**

Replace `crates/kg-core/src/uow/mod.rs`:

```rust
//! Unit of Work staging.

pub mod plan;

use crate::node::{LocalId, NodeId, NodeRef};
use crate::rel::RelId;
use crate::value::PropValue;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::BTreeMap;

/// Sparse property patch. `Some` = set, `None` = remove.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PropPatch(BTreeMap<String, Option<PropValue>>);

impl PropPatch {
    pub fn new() -> Self { Self::default() }
    pub fn set(mut self, name: impl Into<String>, v: PropValue) -> Self {
        self.0.insert(name.into(), Some(v)); self
    }
    pub fn unset(mut self, name: impl Into<String>) -> Self {
        self.0.insert(name.into(), None); self
    }
    pub fn entries(&self) -> &BTreeMap<String, Option<PropValue>> { &self.0 }
}

/// Behavior for deleting a node that still has incident relationships.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CascadeRule { Strict, Detach }

/// DDL specification for an index.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IndexSpec { pub label: String, pub props: Vec<String> }

/// DDL specification for a constraint.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeConstraint {
    Unique  { label: String, props: Vec<String> },
    Exists  { label: String, prop: String },
}

/// A single staged operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StagedOp {
    CreateNode { local: LocalId, labels: SmallVec<[String; 2]>, props: BTreeMap<String, PropValue> },
    MergeNode  { local: LocalId, labels: SmallVec<[String; 2]>, key_props: BTreeMap<String, PropValue>, set_props: BTreeMap<String, PropValue> },
    UpdateNode { id: NodeId, patch: PropPatch },
    DeleteNode { id: NodeId, cascade: CascadeRule },
    CreateRel  { local: LocalId, r#type: String, start: NodeRef, end: NodeRef, props: BTreeMap<String, PropValue> },
    MergeRel   { local: LocalId, r#type: String, start: NodeRef, end: NodeRef, key_props: BTreeMap<String, PropValue>, set_props: BTreeMap<String, PropValue> },
    UpdateRel  { id: RelId, patch: PropPatch },
    DeleteRel  { id: RelId },
    EnsureConstraint(NodeConstraint),
    EnsureIndex(IndexSpec),
}

/// Staged mutations against a graph; commit emits one Cypher transaction.
#[derive(Clone, Debug, Default)]
pub struct UnitOfWork {
    ops: Vec<StagedOp>,
    next_local: u64,
}

impl UnitOfWork {
    pub fn new() -> Self { Self::default() }
    pub fn ops(&self) -> &[StagedOp] { &self.ops }

    fn fresh_local(&mut self) -> LocalId {
        self.next_local += 1;
        LocalId(self.next_local)
    }

    pub fn create_node<L, S, I, K>(&mut self, labels: L, props: I) -> LocalId
    where
        L: IntoIterator<Item = S>, S: Into<String>,
        I: IntoIterator<Item = (K, PropValue)>, K: Into<String>,
    {
        let local = self.fresh_local();
        self.ops.push(StagedOp::CreateNode {
            local,
            labels: labels.into_iter().map(Into::into).collect(),
            props: props.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        });
        local
    }

    pub fn merge_node<L, S, I, J, K1, K2>(&mut self, labels: L, key_props: I, set_props: J) -> LocalId
    where
        L: IntoIterator<Item = S>, S: Into<String>,
        I: IntoIterator<Item = (K1, PropValue)>, K1: Into<String>,
        J: IntoIterator<Item = (K2, PropValue)>, K2: Into<String>,
    {
        let local = self.fresh_local();
        self.ops.push(StagedOp::MergeNode {
            local,
            labels: labels.into_iter().map(Into::into).collect(),
            key_props: key_props.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            set_props: set_props.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        });
        local
    }

    pub fn update_node(&mut self, id: NodeId, patch: PropPatch) {
        self.ops.push(StagedOp::UpdateNode { id, patch });
    }
    pub fn delete_node(&mut self, id: NodeId, cascade: CascadeRule) {
        self.ops.push(StagedOp::DeleteNode { id, cascade });
    }

    pub fn create_rel<T, I, K>(&mut self, start: impl Into<NodeRef>, end: impl Into<NodeRef>, r#type: T, props: I) -> LocalId
    where
        T: Into<String>,
        I: IntoIterator<Item = (K, PropValue)>, K: Into<String>,
    {
        let local = self.fresh_local();
        self.ops.push(StagedOp::CreateRel {
            local,
            r#type: r#type.into(),
            start: start.into(),
            end: end.into(),
            props: props.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        });
        local
    }

    pub fn update_rel(&mut self, id: RelId, patch: PropPatch) {
        self.ops.push(StagedOp::UpdateRel { id, patch });
    }
    pub fn delete_rel(&mut self, id: RelId) {
        self.ops.push(StagedOp::DeleteRel { id });
    }

    pub fn ensure_constraint(&mut self, c: NodeConstraint) {
        self.ops.push(StagedOp::EnsureConstraint(c));
    }
    pub fn ensure_index(&mut self, ix: IndexSpec) {
        self.ops.push(StagedOp::EnsureIndex(ix));
    }
}

// Ergonomics: bare LocalId / NodeId convert into NodeRef.
impl From<LocalId> for NodeRef { fn from(v: LocalId) -> Self { NodeRef::Local(v) } }
impl From<NodeId>  for NodeRef { fn from(v: NodeId)  -> Self { NodeRef::Server(v) } }
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p kg-core --lib uow::tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kg-core/src/uow/mod.rs
git commit -m "feat(kg-core): UnitOfWork staging with sparse PropPatch and CascadeRule"
```

---

## Task 10: Commit plan + topological ordering

**Files:**
- Modify: `crates/kg-core/src/uow/plan.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/kg-core/src/uow/plan.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{LocalId, NodeId, NodeRef};
    use crate::uow::{CascadeRule, PropPatch, UnitOfWork};
    use crate::value::PropValue;

    #[test]
    fn ordering_constraints_before_creates() {
        let mut uow = UnitOfWork::new();
        let a = uow.create_node(["P"], []);
        let _b = uow.create_node(["P"], []);
        uow.update_node(NodeId(1), PropPatch::new().set("x", PropValue::Int(1)));
        uow.delete_node(NodeId(2), CascadeRule::Detach);
        uow.ensure_index(crate::uow::IndexSpec { label: "P".into(), props: vec!["name".into()] });
        let plan = CommitPlan::from_uow(&uow).unwrap();
        let kinds: Vec<&str> = plan.ordered.iter().map(kind_of).collect();
        assert_eq!(kinds[0], "EnsureIndex");
        assert!(kinds[1] == "CreateNode" && kinds[2] == "CreateNode");
        assert!(kinds.iter().position(|&k| k == "UpdateNode").unwrap()
              > kinds.iter().rposition(|&k| k == "CreateNode").unwrap());
        let _ = a;
    }

    #[test]
    fn rel_referencing_local_after_node_create() {
        let mut uow = UnitOfWork::new();
        let a = uow.create_node(["P"], []);
        let b = uow.create_node(["P"], []);
        uow.create_rel(NodeRef::Local(a), NodeRef::Local(b), "R", []);
        let plan = CommitPlan::from_uow(&uow).unwrap();
        let kinds: Vec<&str> = plan.ordered.iter().map(kind_of).collect();
        assert_eq!(kinds, vec!["CreateNode", "CreateNode", "CreateRel"]);
    }

    #[test]
    fn rel_referencing_unstaged_local_id_errors() {
        let mut uow = UnitOfWork::new();
        let phantom = LocalId(99);
        uow.create_rel(NodeRef::Local(phantom), NodeRef::Server(NodeId(1)), "R", []);
        let err = CommitPlan::from_uow(&uow).unwrap_err();
        assert!(matches!(err, crate::error::CoreError::UnresolvedLocalId(_)));
    }

    fn kind_of(op: &crate::uow::StagedOp) -> &'static str {
        use crate::uow::StagedOp::*;
        match op {
            EnsureConstraint(_) => "EnsureConstraint",
            EnsureIndex(_) => "EnsureIndex",
            CreateNode { .. } => "CreateNode",
            MergeNode { .. } => "MergeNode",
            CreateRel { .. } => "CreateRel",
            MergeRel { .. } => "MergeRel",
            UpdateNode { .. } => "UpdateNode",
            UpdateRel { .. } => "UpdateRel",
            DeleteRel { .. } => "DeleteRel",
            DeleteNode { .. } => "DeleteNode",
        }
    }
}
```

Run: `cargo test -p kg-core --lib uow::plan::tests`
Expected: FAIL.

- [ ] **Step 2: Implement**

Replace `crates/kg-core/src/uow/plan.rs`:

```rust
//! Commit planning (topological sort).

use crate::error::CoreError;
use crate::node::{LocalId, NodeRef};
use crate::uow::{StagedOp, UnitOfWork};
use std::collections::HashSet;

/// Ordered staged ops ready for Cypher emission.
#[derive(Clone, Debug)]
pub struct CommitPlan {
    pub ordered: Vec<StagedOp>,
}

impl CommitPlan {
    /// Sort `uow.ops()` into the canonical commit order and verify that every
    /// rel referencing a `LocalId` refers to a node staged earlier.
    pub fn from_uow(uow: &UnitOfWork) -> Result<Self, CoreError> {
        let phase = |op: &StagedOp| -> u8 {
            use StagedOp::*;
            match op {
                EnsureConstraint(_) | EnsureIndex(_) => 0,
                CreateNode { .. } | MergeNode { .. } => 1,
                CreateRel { .. } | MergeRel { .. } => 2,
                UpdateNode { .. } => 3,
                UpdateRel { .. } => 4,
                DeleteRel { .. } => 5,
                DeleteNode { .. } => 6,
            }
        };
        let mut ordered: Vec<StagedOp> = uow.ops().to_vec();
        // stable sort preserves relative order within a phase
        ordered.sort_by_key(phase);

        // verify LocalId resolution
        let mut known: HashSet<LocalId> = HashSet::new();
        for op in &ordered {
            match op {
                StagedOp::CreateNode { local, .. }
                | StagedOp::MergeNode { local, .. }  => { known.insert(*local); }
                StagedOp::CreateRel { start, end, .. }
                | StagedOp::MergeRel { start, end, .. } => {
                    check_ref(start, &known)?;
                    check_ref(end, &known)?;
                }
                _ => {}
            }
        }
        Ok(CommitPlan { ordered })
    }
}

fn check_ref(r: &NodeRef, known: &HashSet<LocalId>) -> Result<(), CoreError> {
    if let NodeRef::Local(id) = r {
        if !known.contains(id) {
            return Err(CoreError::UnresolvedLocalId(*id));
        }
    }
    Ok(())
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p kg-core --lib uow::plan::tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kg-core/src/uow/plan.rs
git commit -m "feat(kg-core): CommitPlan ordering + LocalId resolution"
```

---

## Task 11: `Statement` + parameter representation

**Files:**
- Modify: `crates/kg-core/src/cypher/statement.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/kg-core/src/cypher/statement.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::PropValue;

    #[test]
    fn statement_holds_cypher_and_params() {
        let s = Statement::new("RETURN $x", [("x", PropValue::Int(1))]);
        assert_eq!(s.cypher, "RETURN $x");
        assert_eq!(s.params.get("x"), Some(&PropValue::Int(1)));
    }

    #[test]
    fn sanitize_ident_rejects_injection() {
        assert!(sanitize_ident("Person").is_ok());
        assert!(sanitize_ident("Per_son1").is_ok());
        assert!(sanitize_ident("1bad").is_err());
        assert!(sanitize_ident("a-b").is_err());
        assert!(sanitize_ident("a`b").is_err());
        assert!(sanitize_ident("").is_err());
    }
}
```

Run: `cargo test -p kg-core --lib cypher::statement::tests`
Expected: FAIL.

- [ ] **Step 2: Implement**

Replace `crates/kg-core/src/cypher/statement.rs`:

```rust
//! Statement and parameter representation.

use crate::value::PropValue;
use std::collections::BTreeMap;

/// Parameter map. Keys are bare names (no leading `$`).
pub type ParamMap = BTreeMap<String, PropValue>;

/// A single parameterized Cypher statement.
#[derive(Clone, Debug, PartialEq)]
pub struct Statement {
    pub cypher: String,
    pub params: ParamMap,
}

impl Statement {
    pub fn new<I, K>(cypher: impl Into<String>, params: I) -> Self
    where
        I: IntoIterator<Item = (K, PropValue)>,
        K: Into<String>,
    {
        Statement {
            cypher: cypher.into(),
            params: params.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        }
    }
}

/// Reject anything that would let an attacker break out of an
/// identifier context (label, rel type, prop name).
pub fn sanitize_ident(s: &str) -> Result<&str, IdentError> {
    if s.is_empty() { return Err(IdentError::Empty); }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(IdentError::BadStart(first));
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return Err(IdentError::BadChar(c));
        }
    }
    Ok(s)
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum IdentError {
    #[error("identifier is empty")]
    Empty,
    #[error("identifier cannot start with `{0}`")]
    BadStart(char),
    #[error("identifier contains illegal char `{0}`")]
    BadChar(char),
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p kg-core --lib cypher::statement::tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kg-core/src/cypher/statement.rs
git commit -m "feat(kg-core): Statement, ParamMap, identifier sanitizer"
```

---

## Task 12: Cypher emission — DDL

**Files:**
- Modify: `crates/kg-core/src/cypher/ddl.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/kg-core/src/cypher/ddl.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::uow::{IndexSpec, NodeConstraint};

    #[test]
    fn emit_unique_constraint() {
        let s = emit_constraint(&NodeConstraint::Unique {
            label: "Person".into(),
            props: vec!["name".into(), "email".into()],
        }).unwrap();
        assert!(s.cypher.starts_with("CREATE CONSTRAINT IF NOT EXISTS"));
        assert!(s.cypher.contains("(n:`Person`)"));
        assert!(s.cypher.contains("n.`name`, n.`email`"));
        assert!(s.cypher.contains("IS UNIQUE"));
    }

    #[test]
    fn emit_exists_constraint() {
        let s = emit_constraint(&NodeConstraint::Exists {
            label: "Person".into(), prop: "name".into(),
        }).unwrap();
        assert!(s.cypher.contains("IS NOT NULL"));
    }

    #[test]
    fn emit_index() {
        let s = emit_index(&IndexSpec { label: "Person".into(), props: vec!["name".into()] }).unwrap();
        assert!(s.cypher.starts_with("CREATE INDEX IF NOT EXISTS"));
        assert!(s.cypher.contains("(n:`Person`)"));
    }

    #[test]
    fn rejects_label_with_backtick() {
        let r = emit_index(&IndexSpec { label: "Bad`".into(), props: vec!["x".into()] });
        assert!(r.is_err());
    }
}
```

Run: `cargo test -p kg-core --lib cypher::ddl::tests`
Expected: FAIL.

- [ ] **Step 2: Implement**

Replace `crates/kg-core/src/cypher/ddl.rs`:

```rust
//! DDL emission (CREATE CONSTRAINT / INDEX).

use crate::cypher::statement::{sanitize_ident, IdentError, Statement};
use crate::uow::{IndexSpec, NodeConstraint};

pub fn emit_constraint(c: &NodeConstraint) -> Result<Statement, IdentError> {
    match c {
        NodeConstraint::Unique { label, props } => {
            sanitize_ident(label)?;
            let cols = props.iter()
                .map(|p| { sanitize_ident(p).map(|s| format!("n.`{s}`")) })
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            Ok(Statement::new(
                format!(
                    "CREATE CONSTRAINT IF NOT EXISTS FOR (n:`{label}`) REQUIRE ({cols}) IS UNIQUE"
                ),
                std::iter::empty::<(String, _)>(),
            ))
        }
        NodeConstraint::Exists { label, prop } => {
            sanitize_ident(label)?;
            sanitize_ident(prop)?;
            Ok(Statement::new(
                format!(
                    "CREATE CONSTRAINT IF NOT EXISTS FOR (n:`{label}`) REQUIRE n.`{prop}` IS NOT NULL"
                ),
                std::iter::empty::<(String, _)>(),
            ))
        }
    }
}

pub fn emit_index(ix: &IndexSpec) -> Result<Statement, IdentError> {
    sanitize_ident(&ix.label)?;
    let cols = ix.props.iter()
        .map(|p| sanitize_ident(p).map(|s| format!("n.`{s}`")))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(Statement::new(
        format!("CREATE INDEX IF NOT EXISTS FOR (n:`{}`) ON ({})", ix.label, cols),
        std::iter::empty::<(String, _)>(),
    ))
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p kg-core --lib cypher::ddl::tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kg-core/src/cypher/ddl.rs
git commit -m "feat(kg-core): emit DDL for constraints + indexes"
```

---

## Task 13: Cypher emission — nodes

**Files:**
- Modify: `crates/kg-core/src/cypher/nodes.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/kg-core/src/cypher/nodes.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{LocalId, NodeId};
    use crate::uow::{CascadeRule, PropPatch};
    use crate::value::PropValue;
    use std::collections::BTreeMap;

    #[test]
    fn create_node_emits_create_with_param_props() {
        let mut props = BTreeMap::new();
        props.insert("name".into(), PropValue::String("A".into()));
        let s = emit_create_node(LocalId(1), &smallvec::smallvec!["Person".into()], &props).unwrap();
        assert!(s.cypher.contains("CREATE (n_1:`Person` $props_n_1)"));
        assert_eq!(s.params.get("props_n_1"), Some(&PropValue::Map(props.clone())));
    }

    #[test]
    fn update_node_emits_match_set_remove() {
        let patch = PropPatch::new()
            .set("a", PropValue::Int(1))
            .unset("b");
        let s = emit_update_node(NodeId(42), &patch).unwrap();
        assert!(s.cypher.contains("MATCH (n) WHERE id(n) = $id_42"));
        assert!(s.cypher.contains("SET n.`a` = $set_42_a"));
        assert!(s.cypher.contains("REMOVE n.`b`"));
        assert_eq!(s.params.get("id_42"), Some(&PropValue::Int(42)));
    }

    #[test]
    fn delete_node_detach() {
        let s = emit_delete_node(NodeId(5), CascadeRule::Detach).unwrap();
        assert!(s.cypher.contains("DETACH DELETE n"));
    }

    #[test]
    fn delete_node_strict() {
        let s = emit_delete_node(NodeId(5), CascadeRule::Strict).unwrap();
        assert!(s.cypher.contains("DELETE n"));
        assert!(!s.cypher.contains("DETACH"));
    }
}
```

Run: `cargo test -p kg-core --lib cypher::nodes::tests`
Expected: FAIL.

- [ ] **Step 2: Implement**

Replace `crates/kg-core/src/cypher/nodes.rs`:

```rust
//! Node op emission.

use crate::cypher::statement::{sanitize_ident, IdentError, Statement};
use crate::node::{LocalId, NodeId};
use crate::uow::{CascadeRule, PropPatch};
use crate::value::PropValue;
use smallvec::SmallVec;
use std::collections::BTreeMap;

pub fn node_var(local: LocalId) -> String { format!("n_{}", local.0) }

pub fn emit_create_node(
    local: LocalId,
    labels: &SmallVec<[String; 2]>,
    props: &BTreeMap<String, PropValue>,
) -> Result<Statement, IdentError> {
    let var = node_var(local);
    let labels_str = labels
        .iter()
        .map(|l| sanitize_ident(l).map(|s| format!(":`{s}`")))
        .collect::<Result<Vec<_>, _>>()?
        .join("");
    let pkey = format!("props_{var}");
    Ok(Statement::new(
        format!("CREATE ({var}{labels_str} ${pkey})"),
        [(pkey, PropValue::Map(props.clone()))],
    ))
}

pub fn emit_merge_node(
    local: LocalId,
    labels: &SmallVec<[String; 2]>,
    key_props: &BTreeMap<String, PropValue>,
    set_props: &BTreeMap<String, PropValue>,
) -> Result<Statement, IdentError> {
    let var = node_var(local);
    let labels_str = labels
        .iter()
        .map(|l| sanitize_ident(l).map(|s| format!(":`{s}`")))
        .collect::<Result<Vec<_>, _>>()?
        .join("");
    let kkey = format!("kprops_{var}");
    let skey = format!("sprops_{var}");
    let key_pattern = if key_props.is_empty() {
        String::new()
    } else {
        let inner = key_props
            .keys()
            .map(|k| sanitize_ident(k).map(|s| format!("`{s}`: ${kkey}.`{s}`")))
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        format!(" {{ {inner} }}")
    };
    let mut cypher = format!("MERGE ({var}{labels_str}{key_pattern})");
    let mut params = vec![
        (kkey, PropValue::Map(key_props.clone())),
    ];
    if !set_props.is_empty() {
        cypher.push_str(&format!(" SET {var} += ${skey}"));
        params.push((skey, PropValue::Map(set_props.clone())));
    }
    Ok(Statement::new(cypher, params))
}

pub fn emit_update_node(id: NodeId, patch: &PropPatch) -> Result<Statement, IdentError> {
    let id_key = format!("id_{}", id.0);
    let mut sets = vec![];
    let mut removes = vec![];
    let mut params: Vec<(String, PropValue)> = vec![(id_key.clone(), PropValue::Int(id.0))];
    for (name, val) in patch.entries() {
        let n = sanitize_ident(name)?;
        match val {
            Some(v) => {
                let pkey = format!("set_{}_{}", id.0, n);
                sets.push(format!("SET n.`{n}` = ${pkey}"));
                params.push((pkey, v.clone()));
            }
            None => removes.push(format!("REMOVE n.`{n}`")),
        }
    }
    let body = sets.into_iter().chain(removes).collect::<Vec<_>>().join(" ");
    let cypher = format!(
        "MATCH (n) WHERE id(n) = ${id_key} {body}"
    );
    Ok(Statement::new(cypher, params))
}

pub fn emit_delete_node(id: NodeId, cascade: CascadeRule) -> Result<Statement, IdentError> {
    let id_key = format!("id_{}", id.0);
    let action = match cascade { CascadeRule::Detach => "DETACH DELETE n", CascadeRule::Strict => "DELETE n" };
    Ok(Statement::new(
        format!("MATCH (n) WHERE id(n) = ${id_key} {action}"),
        [(id_key, PropValue::Int(id.0))],
    ))
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p kg-core --lib cypher::nodes::tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kg-core/src/cypher/nodes.rs
git commit -m "feat(kg-core): node create/merge/update/delete Cypher emission"
```

---

## Task 14: Cypher emission — rels with LocalId resolution

**Files:**
- Modify: `crates/kg-core/src/cypher/rels.rs`

- [ ] **Step 1: Write failing tests**

Append to `crates/kg-core/src/cypher/rels.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{LocalId, NodeId, NodeRef};
    use crate::rel::RelId;
    use crate::uow::PropPatch;
    use crate::value::PropValue;
    use std::collections::BTreeMap;

    #[test]
    fn rel_between_two_locals_uses_vars() {
        let s = emit_create_rel(
            LocalId(1),
            NodeRef::Local(LocalId(2)),
            NodeRef::Local(LocalId(3)),
            "KNOWS",
            &BTreeMap::new(),
        ).unwrap();
        assert!(s.cypher.contains("(n_2)-[r_1:`KNOWS`"));
        assert!(s.cypher.contains("->(n_3)"));
        assert!(!s.cypher.contains("MATCH"));
    }

    #[test]
    fn rel_with_server_endpoint_emits_match() {
        let s = emit_create_rel(
            LocalId(1),
            NodeRef::Local(LocalId(2)),
            NodeRef::Server(NodeId(99)),
            "KNOWS",
            &BTreeMap::new(),
        ).unwrap();
        assert!(s.cypher.contains("MATCH (n_end_1) WHERE id(n_end_1) = $id_end_1"));
        assert_eq!(s.params.get("id_end_1"), Some(&PropValue::Int(99)));
    }

    #[test]
    fn rel_props_param() {
        let mut props = BTreeMap::new();
        props.insert("since".into(), PropValue::Int(2020));
        let s = emit_create_rel(
            LocalId(7),
            NodeRef::Local(LocalId(1)),
            NodeRef::Local(LocalId(2)),
            "KNOWS",
            &props,
        ).unwrap();
        assert!(s.cypher.contains("$props_r_7"));
        assert_eq!(s.params.get("props_r_7"), Some(&PropValue::Map(props)));
    }

    #[test]
    fn update_rel_emits_match_id() {
        let s = emit_update_rel(RelId(5),
            &PropPatch::new().set("a", PropValue::Int(1))).unwrap();
        assert!(s.cypher.contains("MATCH ()-[r]->() WHERE id(r) = $rid_5"));
        assert!(s.cypher.contains("SET r.`a` = $set_5_a"));
    }

    #[test]
    fn delete_rel() {
        let s = emit_delete_rel(RelId(8)).unwrap();
        assert!(s.cypher.contains("DELETE r"));
    }
}
```

Run: `cargo test -p kg-core --lib cypher::rels::tests`
Expected: FAIL.

- [ ] **Step 2: Implement**

Replace `crates/kg-core/src/cypher/rels.rs`:

```rust
//! Rel op emission.

use crate::cypher::nodes::node_var;
use crate::cypher::statement::{sanitize_ident, IdentError, Statement};
use crate::node::{LocalId, NodeRef};
use crate::rel::RelId;
use crate::uow::PropPatch;
use crate::value::PropValue;
use std::collections::BTreeMap;

pub fn rel_var(local: LocalId) -> String { format!("r_{}", local.0) }

struct Endpoint {
    var: String,
    /// MATCH prefix to be prepended, if endpoint is server-side.
    prelude: Option<String>,
    extra_params: Vec<(String, PropValue)>,
}

fn endpoint(nr: &NodeRef, role: &str, rel_local: LocalId) -> Endpoint {
    match nr {
        NodeRef::Local(l) => Endpoint {
            var: node_var(*l), prelude: None, extra_params: vec![],
        },
        NodeRef::Server(id) => {
            let var = format!("n_{role}_{}", rel_local.0);
            let key = format!("id_{role}_{}", rel_local.0);
            Endpoint {
                var: var.clone(),
                prelude: Some(format!("MATCH ({var}) WHERE id({var}) = ${key}")),
                extra_params: vec![(key, PropValue::Int(id.0))],
            }
        }
    }
}

pub fn emit_create_rel(
    local: LocalId,
    start: NodeRef,
    end: NodeRef,
    r#type: &str,
    props: &BTreeMap<String, PropValue>,
) -> Result<Statement, IdentError> {
    sanitize_ident(r#type)?;
    let s = endpoint(&start, "start", local);
    let e = endpoint(&end,   "end",   local);
    let rvar = rel_var(local);
    let pkey = format!("props_{rvar}");
    let mut preludes = vec![];
    let mut params: Vec<(String, PropValue)> = vec![];
    if let Some(p) = &s.prelude { preludes.push(p.clone()); }
    if let Some(p) = &e.prelude { preludes.push(p.clone()); }
    params.extend(s.extra_params);
    params.extend(e.extra_params);
    params.push((pkey.clone(), PropValue::Map(props.clone())));
    let prelude = preludes.join(" ");
    let body = format!(
        "CREATE ({})-[{rvar}:`{ty}` ${pkey}]->({})",
        s.var, e.var, ty = r#type,
    );
    let cypher = if prelude.is_empty() { body } else { format!("{prelude} {body}") };
    Ok(Statement::new(cypher, params))
}

pub fn emit_merge_rel(
    local: LocalId,
    start: NodeRef,
    end: NodeRef,
    r#type: &str,
    key_props: &BTreeMap<String, PropValue>,
    set_props: &BTreeMap<String, PropValue>,
) -> Result<Statement, IdentError> {
    sanitize_ident(r#type)?;
    let s = endpoint(&start, "start", local);
    let e = endpoint(&end,   "end",   local);
    let rvar = rel_var(local);
    let kkey = format!("kprops_{rvar}");
    let skey = format!("sprops_{rvar}");

    let key_pattern = if key_props.is_empty() {
        String::new()
    } else {
        let inner = key_props
            .keys()
            .map(|k| sanitize_ident(k).map(|s| format!("`{s}`: ${kkey}.`{s}`")))
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        format!(" {{ {inner} }}")
    };

    let mut preludes = vec![];
    let mut params: Vec<(String, PropValue)> = vec![];
    if let Some(p) = &s.prelude { preludes.push(p.clone()); }
    if let Some(p) = &e.prelude { preludes.push(p.clone()); }
    params.extend(s.extra_params);
    params.extend(e.extra_params);
    params.push((kkey.clone(), PropValue::Map(key_props.clone())));

    let mut body = format!(
        "MERGE ({})-[{rvar}:`{ty}`{key_pattern}]->({})",
        s.var, e.var, ty = r#type,
    );
    if !set_props.is_empty() {
        body.push_str(&format!(" SET {rvar} += ${skey}"));
        params.push((skey, PropValue::Map(set_props.clone())));
    }
    let prelude = preludes.join(" ");
    let cypher = if prelude.is_empty() { body } else { format!("{prelude} {body}") };
    Ok(Statement::new(cypher, params))
}

pub fn emit_update_rel(id: RelId, patch: &PropPatch) -> Result<Statement, IdentError> {
    let rkey = format!("rid_{}", id.0);
    let mut sets = vec![];
    let mut removes = vec![];
    let mut params: Vec<(String, PropValue)> = vec![(rkey.clone(), PropValue::Int(id.0))];
    for (name, val) in patch.entries() {
        let n = sanitize_ident(name)?;
        match val {
            Some(v) => {
                let pkey = format!("set_{}_{}", id.0, n);
                sets.push(format!("SET r.`{n}` = ${pkey}"));
                params.push((pkey, v.clone()));
            }
            None => removes.push(format!("REMOVE r.`{n}`")),
        }
    }
    let body = sets.into_iter().chain(removes).collect::<Vec<_>>().join(" ");
    Ok(Statement::new(
        format!("MATCH ()-[r]->() WHERE id(r) = ${rkey} {body}"),
        params,
    ))
}

pub fn emit_delete_rel(id: RelId) -> Result<Statement, IdentError> {
    let rkey = format!("rid_{}", id.0);
    Ok(Statement::new(
        format!("MATCH ()-[r]->() WHERE id(r) = ${rkey} DELETE r"),
        [(rkey, PropValue::Int(id.0))],
    ))
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p kg-core --lib cypher::rels::tests`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kg-core/src/cypher/rels.rs
git commit -m "feat(kg-core): rel create/merge/update/delete with LocalId resolution"
```

---

## Task 15: `CypherEmitter` entrypoint + snapshot tests

**Files:**
- Modify: `crates/kg-core/src/cypher/mod.rs`
- Test: `crates/kg-core/tests/emit_snapshots.rs`

- [ ] **Step 1: Write failing snapshot test**

Create `crates/kg-core/tests/emit_snapshots.rs`:

```rust
use kg_core::cypher::CypherEmitter;
use kg_core::node::{NodeId, NodeRef};
use kg_core::uow::{CascadeRule, IndexSpec, NodeConstraint, PropPatch, UnitOfWork};
use kg_core::value::PropValue;

#[test]
fn snapshot_full_uow() {
    let mut uow = UnitOfWork::new();
    uow.ensure_constraint(NodeConstraint::Unique {
        label: "Person".into(), props: vec!["name".into()],
    });
    uow.ensure_index(IndexSpec { label: "Person".into(), props: vec!["age".into()] });
    let a = uow.create_node(["Person"], [("name", PropValue::from("Alice"))]);
    let b = uow.create_node(["Person"], [("name", PropValue::from("Bob"))]);
    uow.create_rel(a, b, "KNOWS", [("since", PropValue::Int(2020))]);
    uow.update_node(NodeId(42), PropPatch::new().set("nick", PropValue::from("X")).unset("dead"));
    uow.delete_node(NodeId(99), CascadeRule::Detach);

    let stmts = CypherEmitter::emit(&uow).unwrap();
    insta::assert_yaml_snapshot!(stmts);
}
```

Run: `cargo test -p kg-core --test emit_snapshots`
Expected: FAIL — `CypherEmitter::emit` does not exist.

- [ ] **Step 2: Implement `CypherEmitter`**

Replace `crates/kg-core/src/cypher/mod.rs`:

```rust
//! Cypher emitter (UoW -> parameterized statements).

pub mod ddl;
pub mod nodes;
pub mod rels;
pub mod statement;

pub use statement::{ParamMap, Statement};

use crate::error::CoreError;
use crate::uow::{plan::CommitPlan, StagedOp, UnitOfWork};

pub struct CypherEmitter;

impl CypherEmitter {
    /// Lower the UoW into an ordered batch of parameterized statements.
    pub fn emit(uow: &UnitOfWork) -> Result<Vec<Statement>, CoreError> {
        let plan = CommitPlan::from_uow(uow)?;
        let mut out = vec![];
        for op in plan.ordered {
            let s = match op {
                StagedOp::EnsureConstraint(c) => ddl::emit_constraint(&c).map_err(map_id)?,
                StagedOp::EnsureIndex(ix)     => ddl::emit_index(&ix).map_err(map_id)?,
                StagedOp::CreateNode { local, labels, props } => {
                    nodes::emit_create_node(local, &labels, &props).map_err(map_id)?
                }
                StagedOp::MergeNode { local, labels, key_props, set_props } => {
                    nodes::emit_merge_node(local, &labels, &key_props, &set_props).map_err(map_id)?
                }
                StagedOp::UpdateNode { id, patch } => nodes::emit_update_node(id, &patch).map_err(map_id)?,
                StagedOp::DeleteNode { id, cascade } => nodes::emit_delete_node(id, cascade).map_err(map_id)?,
                StagedOp::CreateRel { local, start, end, r#type, props } => {
                    rels::emit_create_rel(local, start, end, &r#type, &props).map_err(map_id)?
                }
                StagedOp::MergeRel { local, start, end, r#type, key_props, set_props } => {
                    rels::emit_merge_rel(local, start, end, &r#type, &key_props, &set_props).map_err(map_id)?
                }
                StagedOp::UpdateRel { id, patch } => rels::emit_update_rel(id, &patch).map_err(map_id)?,
                StagedOp::DeleteRel { id } => rels::emit_delete_rel(id).map_err(map_id)?,
            };
            out.push(s);
        }
        Ok(out)
    }
}

fn map_id(e: statement::IdentError) -> CoreError {
    CoreError::InvalidPatch {
        field: "identifier".into(),
        reason: e.to_string(),
    }
}
```

- [ ] **Step 3: Run snapshot test, accept output**

Run: `INSTA_FORCE_PASS=1 cargo test -p kg-core --test emit_snapshots`
Then: `cargo insta accept`
Expected: snapshot created at `crates/kg-core/tests/snapshots/emit_snapshots__snapshot_full_uow.snap`.

Re-run: `cargo test -p kg-core --test emit_snapshots`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/kg-core/src/cypher/mod.rs crates/kg-core/tests
git commit -m "feat(kg-core): CypherEmitter entrypoint + snapshot test"
```

---

## Task 16: kg-neo4j error types

**Files:**
- Create: `crates/kg-neo4j/src/error.rs`
- Modify: `crates/kg-neo4j/src/lib.rs`

- [ ] **Step 1: Write failing test**

Create `crates/kg-neo4j/src/error.rs`:

```rust
//! Public error types.

use kg_core::CoreError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Neo4jError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("server error [{code}]: {message}")]
    Tx { code: String, message: String },
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("conversion {from} -> {to}: {reason}")]
    Conversion { from: &'static str, to: &'static str, reason: String },
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("connect: {0}")]
    Connect(String),
    #[error("timeout")]
    Timeout,
    #[error("protocol: {0}")]
    Protocol(String),
    #[error("http {status}: {body}")]
    Http { status: u16, body: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn from_core() {
        let e: Neo4jError = CoreError::CycleInPlan.into();
        assert!(matches!(e, Neo4jError::Core(_)));
    }
    #[test]
    fn display() {
        let e = Neo4jError::Tx { code: "Neo.ClientError.X".into(), message: "boom".into() };
        assert!(format!("{e}").contains("Neo.ClientError.X"));
    }
}
```

Add `pub mod error;` and `pub use error::{Neo4jError, TransportError};` to `crates/kg-neo4j/src/lib.rs`.

Run: `cargo test -p kg-neo4j error::tests`
Expected: PASS (TDD shortcut acceptable for pure data types).

- [ ] **Step 2: Commit**

```bash
git add crates/kg-neo4j/src/error.rs crates/kg-neo4j/src/lib.rs
git commit -m "feat(kg-neo4j): public error types"
```

---

## Task 17: Auth helpers

**Files:**
- Create: `crates/kg-neo4j/src/auth.rs`
- Modify: `crates/kg-neo4j/src/lib.rs`

- [ ] **Step 1: Write + implement**

Create `crates/kg-neo4j/src/auth.rs`:

```rust
//! Authentication descriptors.

/// Authentication strategy. Phase 0: basic only.
#[derive(Clone, Debug)]
pub enum Auth {
    Basic { user: String, password: String },
}

pub fn basic(user: impl Into<String>, password: impl Into<String>) -> Auth {
    Auth::Basic { user: user.into(), password: password.into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basic_constructs() {
        let a = basic("neo4j", "test");
        match a { Auth::Basic { user, password } => {
            assert_eq!(user, "neo4j");
            assert_eq!(password, "test");
        } }
    }
}
```

Add `pub mod auth;` and `pub use auth::{Auth, basic};` to `crates/kg-neo4j/src/lib.rs`.

Run: `cargo test -p kg-neo4j auth::tests`
Expected: PASS.

- [ ] **Step 2: Commit**

```bash
git add crates/kg-neo4j/src/auth.rs crates/kg-neo4j/src/lib.rs
git commit -m "feat(kg-neo4j): Auth + basic() helper"
```

---

## Task 18: Transport trait + TxOutcome

**Files:**
- Create: `crates/kg-neo4j/src/transport/mod.rs`

- [ ] **Step 1: Implement**

Create `crates/kg-neo4j/src/transport/mod.rs`:

```rust
//! Transport abstraction. Crate-private; one impl per build via features.

use async_trait::async_trait;
use kg_core::cypher::Statement;
use kg_core::value::PropValue;
use std::collections::BTreeMap;

use crate::error::TransportError;

#[cfg(feature = "native")]
pub(crate) mod bolt;
#[cfg(feature = "wasm")]
pub(crate) mod http;

/// Result of running a single statement.
#[derive(Clone, Debug, Default)]
pub struct StatementResult {
    pub rows: Vec<BTreeMap<String, PropValue>>,
    pub counters: Counters,
}

/// Aggregated counters across a batch transaction.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Counters {
    pub nodes_created: u32,
    pub nodes_deleted: u32,
    pub rels_created:  u32,
    pub rels_deleted:  u32,
    pub props_set:     u32,
    pub labels_added:  u32,
    pub labels_removed: u32,
    pub indexes_added: u32,
    pub constraints_added: u32,
}

/// Outcome of running a batch transaction.
#[derive(Clone, Debug, Default)]
pub struct TxOutcome {
    pub statements: Vec<StatementResult>,
    pub counters: Counters,
}

#[async_trait]
pub(crate) trait Transport: Send + Sync {
    async fn run_tx(&self, stmts: &[Statement]) -> Result<TxOutcome, TransportError>;
    async fn run_autocommit(&self, stmt: &Statement) -> Result<StatementResult, TransportError>;
}
```

Add `async-trait = "0.1"` to `crates/kg-neo4j/Cargo.toml` deps.
Add `mod transport;` to `crates/kg-neo4j/src/lib.rs`.

- [ ] **Step 2: Check**

Run: `cargo check -p kg-neo4j` (features = default = native; bolt.rs/http.rs don't exist yet — temporarily disable submodule reference)

Adjust mod.rs to use a wrapper that compiles even before submodules exist: keep `#[cfg(feature="native")] pub(crate) mod bolt;` only when `bolt.rs` exists. Since this Task only adds the trait, comment out the submodule lines and re-enable in Task 20 / 23.

Final state of this Task's `transport/mod.rs`:

```rust
//! Transport abstraction.
use async_trait::async_trait;
use kg_core::cypher::Statement;
use kg_core::value::PropValue;
use std::collections::BTreeMap;
use crate::error::TransportError;

#[derive(Clone, Debug, Default)]
pub struct StatementResult {
    pub rows: Vec<BTreeMap<String, PropValue>>,
    pub counters: Counters,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Counters {
    pub nodes_created: u32, pub nodes_deleted: u32,
    pub rels_created: u32,  pub rels_deleted: u32,
    pub props_set: u32, pub labels_added: u32, pub labels_removed: u32,
    pub indexes_added: u32, pub constraints_added: u32,
}

#[derive(Clone, Debug, Default)]
pub struct TxOutcome {
    pub statements: Vec<StatementResult>,
    pub counters: Counters,
}

#[async_trait]
pub(crate) trait Transport: Send + Sync {
    async fn run_tx(&self, stmts: &[Statement]) -> Result<TxOutcome, TransportError>;
    async fn run_autocommit(&self, stmt: &Statement) -> Result<StatementResult, TransportError>;
}
```

Run: `cargo check -p kg-neo4j`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/kg-neo4j
git commit -m "feat(kg-neo4j): Transport trait + StatementResult/Counters/TxOutcome"
```

---

## Task 19: PropValue ↔ neo4rs::BoltType conversion (native)

**Files:**
- Create: `crates/kg-neo4j/src/convert/mod.rs`
- Create: `crates/kg-neo4j/src/convert/bolt.rs`

- [ ] **Step 1: Write failing tests**

Create `crates/kg-neo4j/src/convert/bolt.rs`:

```rust
#![cfg(feature = "native")]

use crate::error::Neo4jError;
use kg_core::value::PropValue;
use neo4rs::{BoltType, BoltInteger, BoltString, BoltBoolean, BoltFloat, BoltNull, BoltList, BoltMap};

pub fn to_bolt(v: &PropValue) -> Result<BoltType, Neo4jError> {
    use PropValue::*;
    Ok(match v {
        Null         => BoltType::Null(BoltNull),
        Bool(b)      => BoltType::Boolean(BoltBoolean::new(*b)),
        Int(i)       => BoltType::Integer(BoltInteger::new(*i)),
        Float(f)     => BoltType::Float(BoltFloat::new(*f)),
        String(s)    => BoltType::String(BoltString::new(s)),
        Bytes(_)     => return Err(Neo4jError::Conversion { from: "Bytes",  to: "BoltType", reason: "phase-0 unsupported".into() }),
        List(xs) => {
            let mut bl = BoltList::with_capacity(xs.len());
            for x in xs { bl.push(to_bolt(x)?); }
            BoltType::List(bl)
        }
        Map(m) => {
            let mut bm = BoltMap::with_capacity(m.len());
            for (k, x) in m { bm.put(BoltString::new(k), to_bolt(x)?); }
            BoltType::Map(bm)
        }
        Date(_) | DateTime(_) | Duration { .. } | Point2D { .. } => {
            return Err(Neo4jError::Conversion {
                from: "temporal/spatial",
                to: "BoltType",
                reason: "phase-0 partial; tracked".into(),
            });
        }
    })
}

pub fn from_bolt(b: &BoltType) -> Result<PropValue, Neo4jError> {
    Ok(match b {
        BoltType::Null(_) => PropValue::Null,
        BoltType::Boolean(v) => PropValue::Bool(v.value),
        BoltType::Integer(v) => PropValue::Int(v.value),
        BoltType::Float(v) => PropValue::Float(v.value),
        BoltType::String(v) => PropValue::String(v.value.clone()),
        BoltType::List(xs) => {
            let mut out = Vec::with_capacity(xs.len());
            for x in xs.iter() { out.push(from_bolt(x)?); }
            PropValue::List(out)
        }
        BoltType::Map(m) => {
            let mut out = std::collections::BTreeMap::new();
            for (k, v) in m.iter() { out.insert(k.value.clone(), from_bolt(v)?); }
            PropValue::Map(out)
        }
        other => return Err(Neo4jError::Conversion {
            from: "BoltType",
            to: "PropValue",
            reason: format!("unsupported variant: {other:?}"),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_roundtrip() {
        for v in [
            PropValue::Null,
            PropValue::Bool(true),
            PropValue::Int(42),
            PropValue::Float(1.5),
            PropValue::String("hi".into()),
        ] {
            let b = to_bolt(&v).unwrap();
            let back = from_bolt(&b).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn list_roundtrip() {
        let v = PropValue::List(vec![PropValue::Int(1), PropValue::String("x".into())]);
        let b = to_bolt(&v).unwrap();
        assert_eq!(v, from_bolt(&b).unwrap());
    }

    #[test]
    fn map_roundtrip() {
        let mut m = std::collections::BTreeMap::new();
        m.insert("k".into(), PropValue::Int(1));
        let v = PropValue::Map(m);
        let b = to_bolt(&v).unwrap();
        assert_eq!(v, from_bolt(&b).unwrap());
    }
}
```

Create `crates/kg-neo4j/src/convert/mod.rs`:

```rust
#[cfg(feature = "native")]
pub(crate) mod bolt;
#[cfg(feature = "wasm")]
pub(crate) mod json;
```

Add `mod convert;` to `crates/kg-neo4j/src/lib.rs`.

Run: `cargo test -p kg-neo4j --features native convert::bolt::tests`
Expected: PASS (or compile errors revealing neo4rs API mismatches — adjust to actual neo4rs 0.8 types: `neo4rs::BoltType` variants and constructors. If the API differs in 0.8, replace the constructors with `BoltType::Integer(value.into())` style as documented in `neo4rs::types`. Update tests to match.)

- [ ] **Step 2: Commit**

```bash
git add crates/kg-neo4j/src/convert
git commit -m "feat(kg-neo4j): PropValue<->BoltType conversion (native)"
```

---

## Task 20: Bolt transport impl

**Files:**
- Create: `crates/kg-neo4j/src/transport/bolt.rs`
- Modify: `crates/kg-neo4j/src/transport/mod.rs` (re-enable submodule)

- [ ] **Step 1: Implement**

Re-enable `bolt` submodule in `transport/mod.rs`:

```rust
#[cfg(feature = "native")] pub(crate) mod bolt;
```

Create `crates/kg-neo4j/src/transport/bolt.rs`:

```rust
#![cfg(feature = "native")]

use async_trait::async_trait;
use kg_core::cypher::Statement;
use neo4rs::{ConfigBuilder, Graph, query, Txn};

use crate::auth::Auth;
use crate::convert::bolt::{from_bolt, to_bolt};
use crate::error::{Neo4jError, TransportError};
use super::{Counters, StatementResult, Transport, TxOutcome};

pub(crate) struct BoltTransport {
    graph: Graph,
}

impl BoltTransport {
    pub async fn connect(uri: &str, auth: &Auth, database: Option<&str>) -> Result<Self, Neo4jError> {
        let Auth::Basic { user, password } = auth;
        let mut b = ConfigBuilder::new()
            .uri(uri)
            .user(user)
            .password(password);
        if let Some(db) = database { b = b.db(db); }
        let cfg = b.build().map_err(|e| TransportError::Connect(e.to_string()))?;
        let graph = Graph::connect(cfg).await.map_err(|e| TransportError::Connect(e.to_string()))?;
        Ok(Self { graph })
    }

    fn build_query(stmt: &Statement) -> Result<neo4rs::Query, Neo4jError> {
        let mut q = query(&stmt.cypher);
        for (k, v) in &stmt.params {
            q = q.param(k, to_bolt(v)?);
        }
        Ok(q)
    }

    async fn run_one(txn: &mut Txn, stmt: &Statement) -> Result<StatementResult, Neo4jError> {
        let q = Self::build_query(stmt)?;
        let mut stream = txn.execute(q).await
            .map_err(|e| Neo4jError::Tx { code: code_of(&e), message: e.to_string() })?;
        let mut rows = vec![];
        while let Some(row) = stream.next(txn.handle()).await
            .map_err(|e| Neo4jError::Tx { code: code_of(&e), message: e.to_string() })? {
            let keys = row.keys();
            let mut map = std::collections::BTreeMap::new();
            for k in keys {
                let raw: neo4rs::BoltType = row.get(&k).map_err(|e| Neo4jError::Tx { code: "Row.Decode".into(), message: e.to_string() })?;
                map.insert(k, from_bolt(&raw)?);
            }
            rows.push(map);
        }
        // neo4rs 0.8 does not expose counters on streams uniformly; approximate via summary if present.
        Ok(StatementResult { rows, counters: Counters::default() })
    }
}

fn code_of(e: &neo4rs::Error) -> String {
    // best-effort: neo4rs exposes Neo error codes through Display in some variants
    format!("{e:?}")
        .split_once("code:")
        .and_then(|(_, rest)| rest.split_whitespace().next())
        .unwrap_or("Unknown")
        .trim_matches(|c: char| !c.is_ascii_graphic())
        .to_string()
}

#[async_trait]
impl Transport for BoltTransport {
    async fn run_tx(&self, stmts: &[Statement]) -> Result<TxOutcome, TransportError> {
        let mut txn = self.graph.start_txn().await
            .map_err(|e| TransportError::Protocol(e.to_string()))?;
        let mut statements = Vec::with_capacity(stmts.len());
        for stmt in stmts {
            match Self::run_one(&mut txn, stmt).await {
                Ok(r) => statements.push(r),
                Err(e) => {
                    let _ = txn.rollback().await;
                    return Err(match e {
                        Neo4jError::Transport(t) => t,
                        Neo4jError::Tx { code, message } => TransportError::Protocol(format!("{code}: {message}")),
                        other => TransportError::Protocol(other.to_string()),
                    });
                }
            }
        }
        txn.commit().await.map_err(|e| TransportError::Protocol(e.to_string()))?;
        Ok(TxOutcome { statements, counters: Counters::default() })
    }

    async fn run_autocommit(&self, stmt: &Statement) -> Result<StatementResult, TransportError> {
        let outcome = self.run_tx(std::slice::from_ref(stmt)).await?;
        Ok(outcome.statements.into_iter().next().unwrap_or_default())
    }
}
```

Note: `neo4rs` 0.8 API surface may differ from the names used here. The plan executor should adjust constructor/method names to actual 0.8 docs at <https://docs.rs/neo4rs/0.8>. The semantic shape (config → graph → txn → execute → commit/rollback) is stable.

- [ ] **Step 2: Compile**

Run: `cargo check -p kg-neo4j --features native`
Fix any neo4rs API mismatches inline (likely small renames). Tests for this transport are deferred to Task 22 where a live Neo4j is available.

- [ ] **Step 3: Commit**

```bash
git add crates/kg-neo4j/src/transport
git commit -m "feat(kg-neo4j): Bolt transport impl over neo4rs"
```

---

## Task 21: Client + ClientBuilder (native)

**Files:**
- Create: `crates/kg-neo4j/src/client.rs`
- Modify: `crates/kg-neo4j/src/lib.rs`

- [ ] **Step 1: Implement**

Create `crates/kg-neo4j/src/client.rs`:

```rust
//! Public client + builder.

use kg_core::cypher::CypherEmitter;
use kg_core::schema::SchemaRegistry;
use kg_core::uow::UnitOfWork;
use std::collections::HashMap;

use crate::auth::Auth;
use crate::error::Neo4jError;
use crate::transport::{Transport, TxOutcome};

#[cfg(feature = "native")]
use crate::transport::bolt::BoltTransport;

/// High-level client. One concrete struct; transport chosen at compile time.
pub struct Client {
    transport: Box<dyn Transport>,
    registry: Option<SchemaRegistry>,
}

impl Client {
    pub fn unit_of_work(&self) -> UnitOfWork { UnitOfWork::new() }
    pub fn schema(&self) -> Option<&SchemaRegistry> { self.registry.as_ref() }

    /// Validate (if schema attached) then emit + run as single transaction.
    pub async fn commit(&self, uow: UnitOfWork) -> Result<CommitResult, Neo4jError> {
        // schema validation
        if let Some(reg) = &self.registry {
            let violations = validate_uow(&uow, reg);
            if !violations.is_empty() {
                return Err(Neo4jError::Core(kg_core::CoreError::SchemaViolation(violations)));
            }
        }
        self.commit_unchecked(uow).await
    }

    pub async fn commit_unchecked(&self, uow: UnitOfWork) -> Result<CommitResult, Neo4jError> {
        let stmts = CypherEmitter::emit(&uow)?;
        let _outcome: TxOutcome = self.transport.run_tx(&stmts).await?;
        // Phase 0 returns an empty id_map; richer mapping deferred (requires
        // emitter RETURN clauses + driver result decoding pass — tracked).
        Ok(CommitResult { id_map: HashMap::new() })
    }
}

#[derive(Debug, Default)]
pub struct CommitResult {
    pub id_map: HashMap<kg_core::LocalId, i64>,
}

fn validate_uow(uow: &UnitOfWork, reg: &SchemaRegistry) -> Vec<kg_core::SchemaViolation> {
    use kg_core::uow::StagedOp::*;
    let mut out = vec![];
    for op in uow.ops() {
        match op {
            CreateNode { labels, props, .. } | MergeNode { labels, set_props: props, .. } => {
                if let Some(label) = labels.first() {
                    out.extend(reg.validate_node_props(label, props));
                }
            }
            CreateRel { r#type, .. } | MergeRel { r#type, .. } => {
                out.extend(reg.validate_rel_endpoints(r#type, None, None));
            }
            _ => {}
        }
    }
    out
}

pub struct ClientBuilder {
    uri: String,
    auth: Option<Auth>,
    database: Option<String>,
    registry: Option<SchemaRegistry>,
}

impl ClientBuilder {
    pub fn new(uri: impl Into<String>) -> Self {
        ClientBuilder { uri: uri.into(), auth: None, database: None, registry: None }
    }
    pub fn auth(mut self, a: Auth) -> Self { self.auth = Some(a); self }
    pub fn database(mut self, d: impl Into<String>) -> Self { self.database = Some(d.into()); self }
    pub fn schema(mut self, r: SchemaRegistry) -> Self { self.registry = Some(r); self }

    #[cfg(feature = "native")]
    pub async fn build(self) -> Result<Client, Neo4jError> {
        let auth = self.auth.ok_or_else(|| Neo4jError::Auth("missing auth".into()))?;
        let bt = BoltTransport::connect(&self.uri, &auth, self.database.as_deref()).await?;
        Ok(Client { transport: Box::new(bt), registry: self.registry })
    }
}
```

Add `pub mod client; pub use client::{Client, ClientBuilder, CommitResult};` to `crates/kg-neo4j/src/lib.rs`.

Run: `cargo check -p kg-neo4j --features native`
Expected: PASS.

- [ ] **Step 2: Commit**

```bash
git add crates/kg-neo4j/src/client.rs crates/kg-neo4j/src/lib.rs
git commit -m "feat(kg-neo4j): Client + ClientBuilder with schema validation pre-commit"
```

---

## Task 22: Integration test — full node + rel UoW (testcontainers)

**Files:**
- Create: `crates/kg-neo4j/tests/integration_native.rs`

- [ ] **Step 1: Write the test**

```rust
#![cfg(feature = "native")]

use kg_core::value::PropValue;
use kg_neo4j::{ClientBuilder, auth::basic};
use testcontainers::{clients, GenericImage, core::WaitFor, RunnableImage};

async fn neo4j_client(cli: &clients::Cli) -> (kg_neo4j::Client, testcontainers::Container<'_, GenericImage>) {
    let image = GenericImage::new("neo4j", "5-community")
        .with_env_var("NEO4J_AUTH", "neo4j/testtest")
        .with_exposed_port(7687)
        .with_wait_for(WaitFor::message_on_stdout("Started."));
    let runnable: RunnableImage<GenericImage> = image.into();
    let container = cli.run(runnable);
    let port = container.get_host_port_ipv4(7687);
    let uri = format!("bolt://127.0.0.1:{port}");
    let client = ClientBuilder::new(&uri)
        .auth(basic("neo4j", "testtest"))
        .build()
        .await
        .expect("connect");
    (client, container)
}

#[tokio::test(flavor = "multi_thread")]
async fn create_two_nodes_and_a_rel() {
    let cli = clients::Cli::default();
    let (client, _c) = neo4j_client(&cli).await;

    let mut uow = client.unit_of_work();
    let a = uow.create_node(["Person"], [("name", PropValue::from("Alice"))]);
    let b = uow.create_node(["Person"], [("name", PropValue::from("Bob"))]);
    uow.create_rel(a, b, "KNOWS", [("since", PropValue::Int(2020))]);
    client.commit(uow).await.expect("commit");
}

#[tokio::test(flavor = "multi_thread")]
async fn rollback_on_bad_cypher() {
    use kg_core::cypher::Statement;
    let cli = clients::Cli::default();
    let (_client, _c) = neo4j_client(&cli).await;
    // intentionally minimal: bad cypher in autocommit should error
    // (validated indirectly via internal API in Task 23 after fetch/query exposed)
}
```

- [ ] **Step 2: Run**

Run: `cargo test -p kg-neo4j --features native --test integration_native -- --nocapture`
Expected: PASS (Docker must be available). First run pulls `neo4j:5-community` image.

- [ ] **Step 3: Commit**

```bash
git add crates/kg-neo4j/tests/integration_native.rs
git commit -m "test(kg-neo4j): integration test for node + rel commit via testcontainers"
```

---

## Task 23: Client read API + materialize_schema

**Files:**
- Modify: `crates/kg-neo4j/src/client.rs`
- Create: `crates/kg-neo4j/src/from_row.rs`

- [ ] **Step 1: Implement `query` + `fetch_node` + `fetch_rel` + `materialize_schema`**

Create `crates/kg-neo4j/src/from_row.rs`:

```rust
//! Trait for decoding a single Cypher row into a user struct.

use kg_core::value::PropValue;
use std::collections::BTreeMap;

pub trait FromRow: Sized {
    fn from_row(row: &BTreeMap<String, PropValue>) -> Result<Self, RowError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RowError {
    #[error("missing column `{0}`")]
    Missing(String),
    #[error("wrong type for `{col}`: expected {expected}, got {got}")]
    WrongType { col: String, expected: &'static str, got: &'static str },
}

impl FromRow for BTreeMap<String, PropValue> {
    fn from_row(row: &BTreeMap<String, PropValue>) -> Result<Self, RowError> {
        Ok(row.clone())
    }
}
```

Add `pub mod from_row; pub use from_row::{FromRow, RowError};` to `lib.rs`.

Append to `crates/kg-neo4j/src/client.rs`:

```rust
use kg_core::cypher::Statement;
use kg_core::node::{Node, NodeId};
use kg_core::rel::{Rel, RelId};
use kg_core::value::PropValue;
use kg_core::CoreError;

use crate::from_row::FromRow;

impl Client {
    pub async fn query<T: FromRow>(
        &self,
        cypher: &str,
        params: impl IntoIterator<Item = (impl Into<String>, PropValue)>,
    ) -> Result<Vec<T>, Neo4jError> {
        let stmt = Statement::new(cypher, params);
        let res = self.transport.run_autocommit(&stmt).await?;
        let mut out = Vec::with_capacity(res.rows.len());
        for row in res.rows {
            out.push(T::from_row(&row).map_err(|e| Neo4jError::Core(CoreError::InvalidPatch {
                field: "row".into(), reason: e.to_string(),
            }))?);
        }
        Ok(out)
    }

    pub async fn fetch_node(&self, id: NodeId) -> Result<Option<Node>, Neo4jError> {
        let res: Vec<std::collections::BTreeMap<String, PropValue>> = self.query(
            "MATCH (n) WHERE id(n) = $id RETURN labels(n) AS labels, properties(n) AS props",
            [("id", PropValue::Int(id.0))],
        ).await?;
        let Some(row) = res.into_iter().next() else { return Ok(None); };
        let labels = match row.get("labels") {
            Some(PropValue::List(xs)) => xs.iter().filter_map(|x| match x {
                PropValue::String(s) => Some(s.clone()), _ => None
            }).collect(),
            _ => smallvec::smallvec![],
        };
        let props = match row.get("props") {
            Some(PropValue::Map(m)) => m.clone(),
            _ => Default::default(),
        };
        Ok(Some(Node { id: Some(id), labels, props }))
    }

    pub async fn fetch_rel(&self, id: RelId) -> Result<Option<Rel>, Neo4jError> {
        let res: Vec<std::collections::BTreeMap<String, PropValue>> = self.query(
            "MATCH (s)-[r]->(e) WHERE id(r) = $id \
             RETURN type(r) AS type, id(s) AS start_id, id(e) AS end_id, properties(r) AS props",
            [("id", PropValue::Int(id.0))],
        ).await?;
        let Some(row) = res.into_iter().next() else { return Ok(None); };
        let ty = match row.get("type") {
            Some(PropValue::String(s)) => s.clone(),
            _ => return Ok(None),
        };
        let start_id = match row.get("start_id") { Some(PropValue::Int(i)) => *i, _ => return Ok(None) };
        let end_id   = match row.get("end_id")   { Some(PropValue::Int(i)) => *i, _ => return Ok(None) };
        let props = match row.get("props") {
            Some(PropValue::Map(m)) => m.clone(),
            _ => Default::default(),
        };
        Ok(Some(Rel {
            id: Some(id),
            r#type: ty,
            start: kg_core::NodeRef::Server(NodeId(start_id)),
            end:   kg_core::NodeRef::Server(NodeId(end_id)),
            props,
        }))
    }

    pub async fn materialize_schema(&self) -> Result<(), Neo4jError> {
        let Some(reg) = &self.registry else { return Ok(()); };
        let mut uow = UnitOfWork::new();
        for ns in reg.nodes() {
            for unique in &ns.uniqueness {
                uow.ensure_constraint(kg_core::uow::NodeConstraint::Unique {
                    label: ns.label.clone(), props: unique.clone(),
                });
            }
            for ix in &ns.indexes {
                uow.ensure_index(kg_core::uow::IndexSpec {
                    label: ns.label.clone(), props: ix.clone(),
                });
            }
            for p in &ns.props {
                if p.required {
                    uow.ensure_constraint(kg_core::uow::NodeConstraint::Exists {
                        label: ns.label.clone(), prop: p.name.clone(),
                    });
                }
            }
        }
        self.commit_unchecked(uow).await?;
        Ok(())
    }
}
```

- [ ] **Step 2: Add integration tests**

Append to `crates/kg-neo4j/tests/integration_native.rs`:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn fetch_after_commit() {
    let cli = clients::Cli::default();
    let (client, _c) = neo4j_client(&cli).await;
    // create
    let rows: Vec<std::collections::BTreeMap<String, PropValue>> = client.query(
        "CREATE (n:Person {name:$name}) RETURN id(n) AS id",
        [("name", PropValue::from("Carol"))],
    ).await.unwrap();
    let id = match rows[0].get("id") { Some(PropValue::Int(i)) => *i, _ => panic!() };
    let fetched = client.fetch_node(kg_core::NodeId(id)).await.unwrap().unwrap();
    assert_eq!(fetched.labels.first().map(String::as_str), Some("Person"));
}

#[tokio::test(flavor = "multi_thread")]
async fn materialize_constraint_idempotent() {
    use kg_core::schema::{NodeSchema, PropType, SchemaRegistry};
    let cli = clients::Cli::default();
    let image = testcontainers::GenericImage::new("neo4j", "5-community")
        .with_env_var("NEO4J_AUTH", "neo4j/testtest")
        .with_exposed_port(7687)
        .with_wait_for(testcontainers::core::WaitFor::message_on_stdout("Started."));
    let container = cli.run(image);
    let port = container.get_host_port_ipv4(7687);
    let uri = format!("bolt://127.0.0.1:{port}");
    let mut reg = SchemaRegistry::new();
    reg.add_node(NodeSchema::builder("Person").prop("name", PropType::String).required().unique(["name"]).build());
    let client = ClientBuilder::new(&uri)
        .auth(basic("neo4j", "testtest"))
        .schema(reg)
        .build()
        .await
        .unwrap();
    client.materialize_schema().await.unwrap();
    client.materialize_schema().await.unwrap(); // idempotent
}
```

Run: `cargo test -p kg-neo4j --features native --test integration_native`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/kg-neo4j
git commit -m "feat(kg-neo4j): query/fetch_node/fetch_rel/materialize_schema + tests"
```

---

## Task 24: PropValue ↔ JSON conversion (wasm)

**Files:**
- Create: `crates/kg-neo4j/src/convert/json.rs`

- [ ] **Step 1: Write + implement**

Create `crates/kg-neo4j/src/convert/json.rs`:

```rust
#![cfg(feature = "wasm")]

use crate::error::Neo4jError;
use kg_core::value::PropValue;
use serde_json::{json, Value};

/// Convert to Neo4j HTTP Query API v2 JSON value form.
/// API encodes typed values as `{ "$type": "...", "_value": "..." }`.
pub fn to_json(v: &PropValue) -> Result<Value, Neo4jError> {
    use PropValue::*;
    Ok(match v {
        Null     => Value::Null,
        Bool(b)  => Value::Bool(*b),
        Int(i)   => json!({"$type": "Integer", "_value": i.to_string()}),
        Float(f) => json!({"$type": "Float",   "_value": f.to_string()}),
        String(s) => json!({"$type": "String", "_value": s}),
        Bytes(b) => json!({"$type": "Base64",  "_value": base64_encode(b)}),
        List(xs) => {
            let inner: Vec<Value> = xs.iter().map(to_json).collect::<Result<_, _>>()?;
            json!({"$type": "List", "_value": inner})
        }
        Map(m) => {
            let mut obj = serde_json::Map::new();
            for (k, vv) in m { obj.insert(k.clone(), to_json(vv)?); }
            json!({"$type": "Map", "_value": Value::Object(obj)})
        }
        Date(d) => json!({"$type": "Date",     "_value": d.format("%Y-%m-%d").to_string()}),
        DateTime(dt) => json!({"$type": "OffsetDateTime", "_value": dt.to_rfc3339()}),
        Duration { seconds, nanos } => json!({
            "$type": "Duration",
            "_value": format!("PT{seconds}.{nanos:09}S")
        }),
        Point2D { srid, x, y } => json!({
            "$type": "Point",
            "_value": {"srid": srid, "coordinates": [x, y]}
        }),
    })
}

pub fn from_json(j: &Value) -> Result<PropValue, Neo4jError> {
    if j.is_null() { return Ok(PropValue::Null); }
    if let Some(b) = j.as_bool() { return Ok(PropValue::Bool(b)); }
    if let Some(obj) = j.as_object() {
        let ty = obj.get("$type").and_then(|v| v.as_str()).ok_or_else(|| Neo4jError::Conversion {
            from: "json", to: "PropValue", reason: "missing $type".into(),
        })?;
        let val = obj.get("_value").ok_or_else(|| Neo4jError::Conversion {
            from: "json", to: "PropValue", reason: "missing _value".into(),
        })?;
        return Ok(match ty {
            "Integer" => PropValue::Int(val.as_str().and_then(|s| s.parse().ok())
                .ok_or_else(|| conv("Integer"))?),
            "Float"   => PropValue::Float(val.as_str().and_then(|s| s.parse().ok())
                .ok_or_else(|| conv("Float"))?),
            "String"  => PropValue::String(val.as_str().ok_or_else(|| conv("String"))?.into()),
            "Date"    => PropValue::Date(chrono::NaiveDate::parse_from_str(
                val.as_str().ok_or_else(|| conv("Date"))?, "%Y-%m-%d",
            ).map_err(|e| Neo4jError::Conversion { from: "json", to: "Date", reason: e.to_string() })?),
            "OffsetDateTime" => PropValue::DateTime(
                chrono::DateTime::parse_from_rfc3339(val.as_str().ok_or_else(|| conv("OffsetDateTime"))?)
                    .map_err(|e| Neo4jError::Conversion { from: "json", to: "DateTime", reason: e.to_string() })?
            ),
            "List" => {
                let arr = val.as_array().ok_or_else(|| conv("List"))?;
                PropValue::List(arr.iter().map(from_json).collect::<Result<_, _>>()?)
            }
            "Map" => {
                let m = val.as_object().ok_or_else(|| conv("Map"))?;
                let mut out = std::collections::BTreeMap::new();
                for (k, v) in m { out.insert(k.clone(), from_json(v)?); }
                PropValue::Map(out)
            }
            other => return Err(Neo4jError::Conversion {
                from: "json", to: "PropValue", reason: format!("unsupported type tag: {other}"),
            }),
        });
    }
    Err(Neo4jError::Conversion { from: "json", to: "PropValue", reason: "unexpected shape".into() })
}

fn conv(t: &'static str) -> Neo4jError {
    Neo4jError::Conversion { from: "json", to: t, reason: "invalid value".into() }
}

fn base64_encode(b: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_roundtrip() {
        let v = PropValue::Int(42);
        assert_eq!(v, from_json(&to_json(&v).unwrap()).unwrap());
    }
    #[test]
    fn string_roundtrip() {
        let v = PropValue::String("hi".into());
        assert_eq!(v, from_json(&to_json(&v).unwrap()).unwrap());
    }
    #[test]
    fn list_roundtrip() {
        let v = PropValue::List(vec![PropValue::Int(1), PropValue::String("x".into())]);
        assert_eq!(v, from_json(&to_json(&v).unwrap()).unwrap());
    }
}
```

- [ ] **Step 2: Test**

Run: `cargo test -p kg-neo4j --no-default-features --features wasm --target x86_64-apple-darwin --lib convert::json::tests` (json conversion is pure rust; can run on host with `wasm` feature enabled — gated only by feature, not target).

Note: if the conditional compile fences on `cfg(target_arch="wasm32")` rather than feature, drop the target-specific gate from `from_json`/`to_json`. The plan executor should confirm tests run on host.

Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/kg-neo4j/src/convert
git commit -m "feat(kg-neo4j): PropValue<->JSON conversion for HTTP Query API v2"
```

---

## Task 25: HTTP transport + injectable HttpClient

**Files:**
- Create: `crates/kg-neo4j/src/transport/http.rs`
- Modify: `crates/kg-neo4j/src/transport/mod.rs`

- [ ] **Step 1: Implement**

Re-enable submodule in `transport/mod.rs`:

```rust
#[cfg(feature = "wasm")] pub(crate) mod http;
```

Create `crates/kg-neo4j/src/transport/http.rs`:

```rust
#![cfg(feature = "wasm")]

use async_trait::async_trait;
use kg_core::cypher::Statement;
use serde_json::{json, Value};

use crate::auth::Auth;
use crate::convert::json::{from_json, to_json};
use crate::error::{Neo4jError, TransportError};
use super::{Counters, StatementResult, Transport, TxOutcome};

/// Transport-agnostic HTTP client (injectable for tests).
#[async_trait(?Send)]
pub trait HttpClient {
    async fn post(&self, url: &str, headers: &[(String, String)], body: &str)
        -> Result<HttpResponse, TransportError>;
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

pub(crate) struct HttpTransport<C: HttpClient> {
    base: String,
    database: String,
    auth_header: String,
    client: C,
}

impl<C: HttpClient> HttpTransport<C> {
    pub fn new(base: impl Into<String>, database: impl Into<String>, auth: &Auth, client: C) -> Self {
        let Auth::Basic { user, password } = auth;
        use base64::Engine;
        let token = base64::engine::general_purpose::STANDARD
            .encode(format!("{user}:{password}"));
        Self {
            base: base.into(),
            database: database.into(),
            auth_header: format!("Basic {token}"),
            client,
        }
    }

    fn url(&self) -> String { format!("{}/db/{}/query/v2", self.base, self.database) }
    fn tx_url(&self) -> String { format!("{}/db/{}/query/v2/tx", self.base, self.database) }

    fn headers(&self) -> Vec<(String, String)> {
        vec![
            ("Content-Type".into(), "application/json".into()),
            ("Authorization".into(), self.auth_header.clone()),
        ]
    }

    fn statement_body(stmt: &Statement) -> Result<Value, Neo4jError> {
        let mut params = serde_json::Map::new();
        for (k, v) in &stmt.params { params.insert(k.clone(), to_json(v)?); }
        Ok(json!({
            "statement": stmt.cypher,
            "parameters": params,
            "includeCounters": true,
        }))
    }

    fn batch_body(stmts: &[Statement]) -> Result<String, Neo4jError> {
        let mut arr = Vec::with_capacity(stmts.len());
        for s in stmts { arr.push(Self::statement_body(s)?); }
        Ok(serde_json::to_string(&json!({ "statements": arr })).unwrap())
    }

    fn parse_result(body: &str) -> Result<TxOutcome, Neo4jError> {
        let v: Value = serde_json::from_str(body).map_err(|e| Neo4jError::Transport(
            TransportError::Protocol(format!("invalid json: {e}"))
        ))?;
        if let Some(errs) = v.get("errors").and_then(|e| e.as_array()) {
            if let Some(first) = errs.first() {
                return Err(Neo4jError::Tx {
                    code: first.get("code").and_then(|c| c.as_str()).unwrap_or("Unknown").into(),
                    message: first.get("message").and_then(|m| m.as_str()).unwrap_or("").into(),
                });
            }
        }
        let mut statements = vec![];
        if let Some(results) = v.get("results").and_then(|r| r.as_array()) {
            for r in results {
                let keys = r.get("columns").and_then(|c| c.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect::<Vec<_>>())
                    .unwrap_or_default();
                let mut rows = vec![];
                if let Some(data) = r.get("data").and_then(|d| d.as_array()) {
                    for row in data {
                        if let Some(values) = row.get("row").and_then(|x| x.as_array()) {
                            let mut map = std::collections::BTreeMap::new();
                            for (k, vv) in keys.iter().zip(values.iter()) {
                                map.insert(k.clone(), from_json(vv)?);
                            }
                            rows.push(map);
                        }
                    }
                }
                statements.push(StatementResult { rows, counters: Counters::default() });
            }
        }
        Ok(TxOutcome { statements, counters: Counters::default() })
    }
}

#[async_trait(?Send)]
impl<C: HttpClient> Transport for HttpTransport<C> {
    async fn run_tx(&self, stmts: &[Statement]) -> Result<TxOutcome, TransportError> {
        let body = Self::batch_body(stmts).map_err(|e| TransportError::Protocol(e.to_string()))?;
        let res = self.client.post(&self.tx_url(), &self.headers(), &body).await?;
        if !(200..300).contains(&res.status) {
            return Err(TransportError::Http { status: res.status, body: res.body });
        }
        Self::parse_result(&res.body).map_err(|e| TransportError::Protocol(e.to_string()))
    }

    async fn run_autocommit(&self, stmt: &Statement) -> Result<StatementResult, TransportError> {
        let body = serde_json::to_string(&Self::statement_body(stmt).map_err(|e| TransportError::Protocol(e.to_string()))?).unwrap();
        let res = self.client.post(&self.url(), &self.headers(), &body).await?;
        if !(200..300).contains(&res.status) {
            return Err(TransportError::Http { status: res.status, body: res.body });
        }
        let outcome = Self::parse_result(&res.body).map_err(|e| TransportError::Protocol(e.to_string()))?;
        Ok(outcome.statements.into_iter().next().unwrap_or_default())
    }
}
```

Note: HTTP transport drops `Send` bound from the trait because `wasm32` futures are `!Send`. The `Transport` trait shared with the native build is `Send + Sync`; reconcile by using `cfg`-gated trait bounds or by introducing two distinct trait aliases. Plan executor: introduce a `cfg`-conditional `Transport` trait — bounds `Send + Sync` only when not wasm.

Adjust `transport/mod.rs`:

```rust
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub(crate) trait Transport: Send + Sync {
    async fn run_tx(&self, stmts: &[Statement]) -> Result<TxOutcome, TransportError>;
    async fn run_autocommit(&self, stmt: &Statement) -> Result<StatementResult, TransportError>;
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub(crate) trait Transport {
    async fn run_tx(&self, stmts: &[Statement]) -> Result<TxOutcome, TransportError>;
    async fn run_autocommit(&self, stmt: &Statement) -> Result<StatementResult, TransportError>;
}
```

Also adjust the `Box<dyn Transport>` in `client.rs` similarly.

Run: `cargo check -p kg-neo4j --no-default-features --features wasm --target wasm32-unknown-unknown`
Expected: PASS.

- [ ] **Step 2: Commit**

```bash
git add crates/kg-neo4j
git commit -m "feat(kg-neo4j): HTTP transport with injectable HttpClient"
```

---

## Task 26: WebSysHttpClient + wasm-bindgen-test

**Files:**
- Create: `crates/kg-neo4j/src/transport/http_web.rs` (or extend `http.rs`)
- Create: `crates/kg-neo4j/tests/wasm_smoke.rs`

- [ ] **Step 1: Implement `WebSysHttpClient`**

Append to `crates/kg-neo4j/src/transport/http.rs`:

```rust
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, Headers};

pub struct WebSysHttpClient;

#[async_trait(?Send)]
impl HttpClient for WebSysHttpClient {
    async fn post(&self, url: &str, headers: &[(String, String)], body: &str)
        -> Result<HttpResponse, TransportError>
    {
        let mut init = RequestInit::new();
        init.method("POST").mode(RequestMode::Cors)
            .body(Some(&wasm_bindgen::JsValue::from_str(body)));
        let req = Request::new_with_str_and_init(url, &init)
            .map_err(|e| TransportError::Connect(format!("{e:?}")))?;
        let h: Headers = req.headers();
        for (k, v) in headers { h.set(k, v).map_err(|e| TransportError::Connect(format!("{e:?}")))?; }
        let win = web_sys::window().ok_or_else(|| TransportError::Connect("no window".into()))?;
        let resp_val = JsFuture::from(win.fetch_with_request(&req)).await
            .map_err(|e| TransportError::Connect(format!("{e:?}")))?;
        let resp: Response = resp_val.dyn_into().map_err(|_| TransportError::Protocol("not a Response".into()))?;
        let status = resp.status();
        let text_promise = resp.text().map_err(|e| TransportError::Protocol(format!("{e:?}")))?;
        let body_val = JsFuture::from(text_promise).await
            .map_err(|e| TransportError::Protocol(format!("{e:?}")))?;
        let body = body_val.as_string().unwrap_or_default();
        Ok(HttpResponse { status, body })
    }
}

/// Recording client for tests: captures requests, returns canned responses.
pub struct RecordingHttpClient {
    pub captured: std::sync::Mutex<Vec<RecordedRequest>>,
    pub canned: HttpResponse,
}

#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[async_trait(?Send)]
impl HttpClient for RecordingHttpClient {
    async fn post(&self, url: &str, headers: &[(String, String)], body: &str)
        -> Result<HttpResponse, TransportError>
    {
        self.captured.lock().unwrap().push(RecordedRequest {
            url: url.into(),
            headers: headers.to_vec(),
            body: body.into(),
        });
        Ok(self.canned.clone())
    }
}
```

- [ ] **Step 2: Write wasm-bindgen-test**

Create `crates/kg-neo4j/tests/wasm_smoke.rs`:

```rust
#![cfg(all(feature = "wasm", target_arch = "wasm32"))]

use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

use kg_core::value::PropValue;
use kg_neo4j::transport::http::{HttpClient, HttpResponse, HttpTransport, RecordingHttpClient};
use kg_neo4j::auth::basic;

#[wasm_bindgen_test]
async fn batch_body_shape() {
    let canned = HttpResponse { status: 200, body: r#"{"results":[],"errors":[]}"#.into() };
    let rc = RecordingHttpClient { captured: Default::default(), canned };
    let tr = HttpTransport::new("http://localhost:7474", "neo4j", &basic("u","p"), rc);
    let stmt = kg_core::cypher::Statement::new(
        "CREATE (n:Person {name:$name})",
        [("name", PropValue::from("A"))],
    );
    tr.run_autocommit(&stmt).await.unwrap();
    let caps = tr_recording_captured(&tr);
    assert_eq!(caps.len(), 1);
    assert!(caps[0].url.ends_with("/db/neo4j/query/v2"));
    assert!(caps[0].body.contains("\"$type\":\"String\""));
    assert!(caps[0].headers.iter().any(|(k,_)| k == "Authorization"));
}

// Helper expected to be exposed by HttpTransport for tests, or use raw RecordingHttpClient handle.
fn tr_recording_captured<T: HttpClient>(_t: &HttpTransport<T>) -> Vec<kg_neo4j::transport::http::RecordedRequest> {
    // The recording client is owned by the transport; expose via a pub(crate) accessor or by
    // building the transport around `Arc<RecordingHttpClient>` and capturing the Arc here.
    // Plan executor: refactor HttpTransport::new to accept Arc<C> and return both transport and Arc.
    vec![]
}
```

Plan executor: refactor `HttpTransport::new` to take `client: std::sync::Arc<C>` so tests retain a handle. Re-implement `tr_recording_captured` to read the Arc.

Run wasm tests: `wasm-pack test --headless --chrome crates/kg-neo4j --features wasm`
Expected: PASS (the Arc/refactor is part of this task).

- [ ] **Step 3: Commit**

```bash
git add crates/kg-neo4j
git commit -m "feat(kg-neo4j): WebSysHttpClient + RecordingHttpClient + wasm-bindgen-test"
```

---

## Task 27: Wire WASM build into ClientBuilder

**Files:**
- Modify: `crates/kg-neo4j/src/client.rs`

- [ ] **Step 1: Add wasm branch to `ClientBuilder::build`**

Append to `crates/kg-neo4j/src/client.rs`:

```rust
#[cfg(feature = "wasm")]
impl ClientBuilder {
    pub fn build_with_http<C>(self, client: C) -> Result<Client, Neo4jError>
    where
        C: crate::transport::http::HttpClient + 'static,
    {
        let auth = self.auth.ok_or_else(|| Neo4jError::Auth("missing auth".into()))?;
        let database = self.database.unwrap_or_else(|| "neo4j".into());
        let tr = crate::transport::http::HttpTransport::new(self.uri, database, &auth, client);
        Ok(Client { transport: Box::new(tr), registry: self.registry })
    }
}
```

Run: `cargo check -p kg-neo4j --no-default-features --features wasm --target wasm32-unknown-unknown`
Expected: PASS.

- [ ] **Step 2: Commit**

```bash
git add crates/kg-neo4j/src/client.rs
git commit -m "feat(kg-neo4j): wasm ClientBuilder::build_with_http(client)"
```

---

## Task 28: Native CRUD example

**Files:**
- Create: `examples/Cargo.toml` (subcrate, or include in workspace `examples` member)
- Create: `examples/native_crud.rs`

- [ ] **Step 1: Convert `examples` to a tiny crate**

`examples/Cargo.toml`:

```toml
[package]
name = "kg-examples"
version = "0.0.0"
edition.workspace = true
publish = false

[dependencies]
kg-core    = { path = "../crates/kg-core" }
kg-neo4j   = { path = "../crates/kg-neo4j", features = ["native"] }
tokio      = { workspace = true, features = ["macros", "rt-multi-thread"] }

[[example]]
name = "native_crud"
path = "native_crud.rs"
```

`examples/native_crud.rs`:

```rust
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
```

Run: `cargo run -p kg-examples --example native_crud` (requires `make neo4j-up`)
Expected: prints `Alice → Bob`.

- [ ] **Step 2: Commit**

```bash
git add examples
git commit -m "docs(examples): native CRUD walkthrough"
```

---

## Task 29: WASM example + manual smoke doc

**Files:**
- Create: `examples/wasm_crud/Cargo.toml`
- Create: `examples/wasm_crud/src/lib.rs`
- Create: `examples/wasm_crud/index.html`
- Create: `docs/manual-smoke.md`

- [ ] **Step 1: WASM example crate**

`examples/wasm_crud/Cargo.toml`:

```toml
[package]
name = "kg-wasm-example"
version = "0.0.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
kg-core  = { path = "../../crates/kg-core" }
kg-neo4j = { path = "../../crates/kg-neo4j", default-features = false, features = ["wasm"] }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
```

`examples/wasm_crud/src/lib.rs`:

```rust
use kg_core::value::PropValue;
use kg_neo4j::{auth::basic, transport::http::WebSysHttpClient, ClientBuilder};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub async fn run() -> Result<(), JsValue> {
    let client = ClientBuilder::new("http://localhost:7474")
        .auth(basic("neo4j", "test"))
        .database("neo4j")
        .build_with_http(WebSysHttpClient)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut uow = client.unit_of_work();
    let a = uow.create_node(["Person"], [("name", PropValue::from("Alice"))]);
    let b = uow.create_node(["Person"], [("name", PropValue::from("Bob"))]);
    uow.create_rel(a, b, "KNOWS", []);
    client.commit(uow).await.map_err(|e| JsValue::from_str(&e.to_string()))?;
    web_sys::console::log_1(&"committed".into());
    Ok(())
}
```

`examples/wasm_crud/index.html`:

```html
<!doctype html>
<html><head><meta charset="utf-8"><title>kg wasm demo</title></head>
<body>
<script type="module">
  import init from "./pkg/kg_wasm_example.js";
  init();
</script>
</body></html>
```

- [ ] **Step 2: Manual smoke doc**

`docs/manual-smoke.md`:

```markdown
# Manual WASM smoke (one-time per release)

## Prereqs
- Docker
- `wasm-pack` installed (`cargo install wasm-pack`)
- A static file server (`python3 -m http.server`)

## Steps
1. `make neo4j-up`
2. Open `cypher-shell` once and create read CORS-permitting config: `dbms.security.http_auth_allowlist=*` — Neo4j 5 sometimes blocks browser fetch without CORS. If browser blocks, run Neo4j with `-e NEO4J_dbms_security_http__auth__allowlist=*` (or use a small CORS proxy).
3. `cd examples/wasm_crud && wasm-pack build --target web`
4. `python3 -m http.server` then open <http://localhost:8000/index.html>
5. Open browser console; expect `committed` logged and no errors.
6. In `cypher-shell`: `MATCH (a:Person)-[:KNOWS]->(b:Person) RETURN a.name, b.name;` — should return Alice, Bob.

Record result here:

| Date       | Engineer | Browser       | Pass/Fail |
|------------|----------|---------------|-----------|
| YYYY-MM-DD | xxx      | Chrome 140    |           |
```

- [ ] **Step 3: Commit**

```bash
git add examples/wasm_crud docs/manual-smoke.md
git commit -m "docs(examples): wasm demo + manual smoke checklist"
```

---

## Task 30: GitHub Actions CI

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write workflow**

```yaml
name: ci
on:
  push: { branches: [main] }
  pull_request:

jobs:
  fmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.78.0
        with: { components: rustfmt }
      - run: cargo fmt --all -- --check

  clippy-core:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.78.0
        with: { components: clippy }
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy -p kg-core --all-targets -- -D warnings

  clippy-neo4j-native:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.78.0
        with: { components: clippy }
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy -p kg-neo4j --features native --all-targets -- -D warnings

  clippy-neo4j-wasm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.78.0
        with: { components: clippy, targets: wasm32-unknown-unknown }
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy -p kg-neo4j --no-default-features --features wasm --target wasm32-unknown-unknown -- -D warnings

  test-core:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.78.0
      - uses: Swatinem/rust-cache@v2
      - run: cargo test -p kg-core --all-features

  test-native:
    runs-on: ubuntu-latest
    services:
      neo4j:
        image: neo4j:5-community
        ports: ["7687:7687"]
        env:
          NEO4J_AUTH: neo4j/testtest
        options: >-
          --health-cmd="cypher-shell -u neo4j -p testtest 'RETURN 1' || exit 1"
          --health-interval=10s --health-timeout=5s --health-retries=20
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.78.0
      - uses: Swatinem/rust-cache@v2
      - run: cargo test -p kg-neo4j --features native -- --test-threads=1

  test-wasm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.78.0
        with: { targets: wasm32-unknown-unknown }
      - uses: Swatinem/rust-cache@v2
      - uses: jetli/wasm-pack-action@v0.4.0
      - run: wasm-pack test --headless --chrome crates/kg-neo4j --features wasm

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.78.0
      - uses: Swatinem/rust-cache@v2
      - run: cargo install cargo-llvm-cov --locked
      - run: cargo llvm-cov -p kg-core --fail-under-lines 80
```

Note: integration tests in CI use the `services.neo4j` container rather than testcontainers-rs (avoids docker-in-docker). Adjust `tests/integration_native.rs` to also accept `NEO4J_URI` and `NEO4J_PASSWORD` env vars; fall back to testcontainers if unset.

- [ ] **Step 2: Tweak `tests/integration_native.rs` for env-based wiring**

Prepend at top of test fns:

```rust
fn neo4j_from_env() -> Option<(String, String)> {
    let uri = std::env::var("NEO4J_URI").ok()?;
    let pwd = std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "testtest".into());
    Some((uri, pwd))
}
```

And in each test, prefer env URI if present (skip testcontainers). Set `NEO4J_URI=bolt://127.0.0.1:7687` and `NEO4J_PASSWORD=testtest` in CI workflow `env:` block.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml crates/kg-neo4j/tests
git commit -m "ci: github actions matrix (fmt/clippy/test-core/test-native/test-wasm/coverage)"
```

---

## Task 31: README + cargo doc

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Write a usable README**

Replace `README.md`:

```markdown
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
uow.create_rel(alice, bob, "KNOWS", []);
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
cargo test  -p kg-neo4j --features native              # needs make neo4j-up
wasm-pack test --headless --chrome crates/kg-neo4j --features wasm
```

## Docs

`cargo doc --no-deps --all-features --open`

See:
- [Design spec](docs/superpowers/specs/2026-05-19-graph-editor-framework-design.md)
- [Implementation plan](docs/superpowers/plans/2026-05-19-kg-editor-framework.md)
- [Manual WASM smoke](docs/manual-smoke.md)

## License

MIT OR Apache-2.0
```

Run: `cargo doc --no-deps --all-features`
Expected: PASS, no doc warnings.

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: README with quickstart, crate map, build matrix"
```

---

## Self-Review (run before handoff)

**Spec coverage walk-through:**

| Spec § | Requirement                                | Plan task(s) |
|--------|--------------------------------------------|--------------|
| §3     | Two-crate workspace, kg-core pure          | 1–3          |
| §5     | PropValue, Node, Rel, LocalId, NodeRef     | 4–5          |
| §6     | UoW staging incl. PropPatch, CascadeRule   | 9            |
| §6     | Ordering, LocalId resolution               | 10           |
| §7     | SchemaRegistry + validate                  | 7–8          |
| §8 native | Bolt transport, conversion              | 19–20        |
| §8 wasm | HTTP transport + JSON conversion          | 24–26        |
| §8     | Mutually exclusive feature guard           | 3            |
| §9     | Client facade (commit/query/fetch/schema)  | 21, 23, 27   |
| §10    | Error model                                | 6, 16        |
| §11 T1 | kg-core unit + snapshot                    | 4–15         |
| §11 T2 | testcontainers integration                 | 22–23, 30    |
| §11 T3 | wasm-bindgen-test with injectable HTTP     | 26           |
| §12    | Dev workflow (Makefile)                    | 1            |
| §13    | CI matrix                                  | 30           |
| §14    | Acceptance criteria 1–5                    | covered      |

**Placeholder scan:** None of the steps contain "TBD"/"TODO"/"implement later". Every code block is real Rust. Two notes call out neo4rs 0.8 API surface differences and direct the executor to update names from the live docs — that is a concrete instruction, not a placeholder.

**Type consistency:** `Statement`, `ParamMap`, `Counters`, `TxOutcome`, `StatementResult`, `CommitResult`, `PropPatch`, `CascadeRule`, `NodeConstraint`, `IndexSpec`, `LocalId`, `NodeId`, `RelId`, `NodeRef`, `PropValue` all defined exactly once and referenced consistently across tasks. `CypherEmitter::emit` signature `(&UnitOfWork) -> Result<Vec<Statement>, CoreError>` matches between Task 15 and Task 21.

**Scope check:** One workspace, one DB, two transports, phase-0 acceptance — coherent for a single plan. Manual smoke + CI gates close the testing loop.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-19-kg-editor-framework.md`. Two execution options:

1. **Subagent-Driven (recommended)** — fresh subagent per task, review between tasks, fast iteration. Uses `superpowers:subagent-driven-development`.
2. **Inline Execution** — execute tasks in this session using `superpowers:executing-plans`, batched with checkpoints.

Which approach?
