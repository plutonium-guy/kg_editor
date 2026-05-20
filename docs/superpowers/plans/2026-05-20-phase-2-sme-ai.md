# Phase 2 — SME UI + AI MCP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a SME-friendly editor + AI MCP integration on top of phase-1. New `kg-schema` crate parses YAML at server startup, `kg-server` gains entity/link/search endpoints, `kg-mcp` exposes tools over stdio for AI agents, `webui` is rewritten around schema-driven forms.

**Architecture:** YAML schema → loaded once at boot → drives both server-side validation and client-side form generation. UI POSTs each pending op to discrete REST endpoints (`/entities`, `/links`) — no WASM staging on the SME path. AI agents call kg-mcp which translates tool calls into the same REST calls.

**Tech Stack:** Rust 1.95, serde_yaml, axum 0.7, reqwest, tokio, jsonrpc-stdio (or rust-mcp-sdk), Vite, React 18, TypeScript strict, react-router-dom 6, react-hook-form, zod, @tanstack/react-query, shadcn/ui (vendored), Cytoscape.js, Playwright.

**Spec:** `docs/superpowers/specs/2026-05-20-phase-2-sme-ai-design.md`

**Pre-task: branch off phase-1**
```bash
git checkout phase-1/ui
git checkout -b phase-2/sme-ai
```

---

## File Structure

```
kg_editor/
├── Cargo.toml                                     # add kg-schema, kg-mcp to members; add serde_yaml dep
├── kg-schema.yaml                                 # NEW: example schema file
├── crates/
│   ├── kg-core/                                   # unchanged
│   ├── kg-neo4j/                                  # unchanged
│   ├── kg-core-wasm/                              # unchanged
│   ├── kg-schema/                                 # NEW
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                             # SchemaFile, parse YAML, to_core_registry
│   │       ├── field.rs                           # FieldSpec, FieldType
│   │       └── error.rs                           # SchemaError
│   ├── kg-server/                                 # EXTENDED
│   │   ├── src/
│   │   │   ├── lib.rs                             # add SchemaFile to AppState
│   │   │   ├── config.rs                          # add KG_SCHEMA env
│   │   │   ├── state.rs                           # add schema: Arc<SchemaFile>
│   │   │   └── routes/
│   │   │       ├── schema.rs                      # NEW: GET /schema
│   │   │       ├── entities.rs                    # NEW: CRUD on nodes
│   │   │       ├── links.rs                       # NEW: CRUD on rels
│   │   │       └── search.rs                      # NEW: full-text search
│   │   └── tests/
│   │       └── server.rs                          # extended
│   └── kg-mcp/                                    # NEW
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs                            # JSON-RPC stdio loop
│           ├── server.rs                          # MCP protocol handler
│           ├── tools.rs                           # 7 tool implementations
│           └── client.rs                          # kg-server REST client
├── webui/                                         # REWRITE most of src/
│   ├── package.json                               # add react-router, react-hook-form, zod, @tanstack/react-query
│   ├── src/
│   │   ├── main.tsx
│   │   ├── App.tsx                                # router shell
│   │   ├── routes/
│   │   │   ├── BrowsePage.tsx                     # sidebar + entity list
│   │   │   ├── EntityPage.tsx                     # entity view + edit
│   │   │   ├── GraphPage.tsx                      # filtered cytoscape
│   │   │   └── SearchPage.tsx
│   │   ├── components/
│   │   │   ├── EntityForm.tsx                     # schema-driven form generator
│   │   │   ├── EntityList.tsx
│   │   │   ├── EntityListSidebar.tsx
│   │   │   ├── RelationshipPicker.tsx
│   │   │   ├── PendingPanel.tsx
│   │   │   ├── PropsCard.tsx
│   │   │   ├── RelsCard.tsx
│   │   │   ├── GraphCanvas.tsx                    # rewritten from phase-1 Canvas
│   │   │   └── ui/                                # shadcn vendored components
│   │   ├── kg/
│   │   │   ├── client.ts                          # rewritten: REST wrappers for new endpoints
│   │   │   ├── schema.ts                          # types + zod generator from FieldSpec
│   │   │   └── pending.ts                         # pending ops queue + commit-all
│   │   ├── state/
│   │   │   └── store.ts                           # rewritten: pending ops + selection
│   │   ├── hooks/
│   │   │   ├── useSchema.ts
│   │   │   ├── useEntities.ts
│   │   │   └── useEntity.ts
│   │   └── styles/
│   └── tests/                                     # playwright
│       └── e2e/
│           └── sme-workflow.spec.ts
├── docs/
│   ├── manual-ui-smoke.md                         # extended for SME flows
│   └── manual-mcp-smoke.md                        # NEW
└── .github/workflows/ci.yml                       # add kg-schema, kg-mcp, playwright jobs
```

---

## Task 1: Workspace updates + branch

**Files:**
- Modify: `Cargo.toml`
- Modify: `.gitignore`

- [ ] **Step 1: Add new members to root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members  = ["crates/kg-core", "crates/kg-neo4j", "crates/kg-core-wasm", "crates/kg-server", "crates/kg-schema", "crates/kg-mcp", "examples"]
exclude  = ["examples/wasm_crud", "webui"]
```

In `[workspace.dependencies]` add:

```toml
serde_yaml   = "0.9"
# kg-mcp uses one of these for MCP protocol — pick one in T11:
#   official: rmcp = "0.x"
#   alternative: jsonrpc + custom
```

- [ ] **Step 2: Append to `.gitignore`**

```
crates/kg-mcp/target/
crates/kg-schema/target/
```

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml .gitignore
git commit -m "chore: workspace updates for phase-2 (kg-schema, kg-mcp)"
```

---

## Task 2: kg-schema crate scaffold

**Files:**
- Create: `crates/kg-schema/Cargo.toml`
- Create: `crates/kg-schema/src/lib.rs`
- Create: `crates/kg-schema/src/field.rs`
- Create: `crates/kg-schema/src/error.rs`

- [ ] **Step 1: `Cargo.toml`**

```toml
[package]
name = "kg-schema"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "YAML-driven schema loader and richer FieldSpec layered on kg-core::SchemaRegistry."

[dependencies]
kg-core    = { path = "../kg-core" }
serde      = { workspace = true }
serde_yaml = { workspace = true }
serde_json = { workspace = true }
thiserror  = { workspace = true }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Empty stubs**

`src/lib.rs`:
```rust
//! YAML-driven schema loader.

pub mod error;
pub mod field;

pub use error::SchemaError;
pub use field::{FieldSpec, FieldType};
```

`src/field.rs`, `src/error.rs`: each a single `//!` doc comment.

- [ ] **Step 3: Build**

```bash
cargo check -p kg-schema
```

- [ ] **Step 4: Commit**

```bash
git add crates/kg-schema
git commit -m "feat(kg-schema): scaffold crate with empty modules"
```

---

## Task 3: kg-schema — FieldSpec + FieldType + error

**Files:**
- Modify: `crates/kg-schema/src/field.rs`
- Modify: `crates/kg-schema/src/error.rs`

- [ ] **Step 1: `src/field.rs`**

```rust
//! FieldSpec + FieldType — richer than kg-core::schema::PropType because
//! it carries enum value lists and ref-to-label info needed by the UI.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldType {
    String,
    Int,
    Float,
    Bool,
    Date,
    DateTime,
    Enum { values: Vec<String> },
    #[serde(rename = "ref")]
    Ref  { label: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldSpec {
    pub name: String,
    #[serde(flatten)]
    pub ty: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}
```

Note: YAML form `- name: { type: string, required: true }` does NOT match the
`#[serde(flatten)]` shape if `name` is the key. The YAML format chosen in the
spec uses a map shape:
```yaml
props:
  - name:    { type: string, required: true }
```
Here each list entry is a single-key map where the key is the field name and
the value carries the rest. That's not directly deserializable to a struct.
Implementation will parse this via a custom helper in Task 4. For now,
`FieldSpec` is the in-memory representation. Adjust YAML in Task 4 if helpful
(consider switching to `- {name: foo, type: string, required: true}` as
canonical form to simplify parsing).

- [ ] **Step 2: `src/error.rs`**

```rust
//! Schema parsing errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("schema invalid: {0}")]
    Invalid(String),
}
```

- [ ] **Step 3: Test + commit**

```bash
cargo check -p kg-schema
git add crates/kg-schema/src/field.rs crates/kg-schema/src/error.rs
git commit -m "feat(kg-schema): FieldSpec, FieldType, SchemaError"
```

---

## Task 4: kg-schema — SchemaFile parser

**Files:**
- Modify: `crates/kg-schema/src/lib.rs`
- Create: `kg-schema.yaml` (top-level)

- [ ] **Step 1: Canonicalize YAML format in `kg-schema.yaml`**

```yaml
nodes:
  Person:
    description: An individual person.
    props:
      - { name: name,   type: string,  required: true, unique: true }
      - { name: age,    type: int }
      - { name: role,   type: enum, values: [engineer, designer, manager, cto, other] }
      - { name: email,  type: string }
      - { name: joined, type: date }
    indexes:
      - [name]
  Company:
    description: A company that employs people.
    props:
      - { name: name,    type: string, required: true, unique: true }
      - { name: founded, type: int }
      - { name: website, type: string }
  Skill:
    description: A discrete skill or competency.
    props:
      - { name: name,     type: string, required: true, unique: true }
      - { name: category, type: enum, values: [language, framework, soft, domain] }

rels:
  KNOWS:
    endpoints: [[Person, Person]]
    cardinality: many_to_many
    props:
      - { name: since,    type: int }
      - { name: strength, type: float }
  WORKS_AT:
    endpoints: [[Person, Company]]
    cardinality: many_to_many
    props:
      - { name: role,       type: string }
      - { name: start_date, type: date }
      - { name: end_date,   type: date }
  HAS_SKILL:
    endpoints: [[Person, Skill]]
    cardinality: many_to_many
    props:
      - { name: level, type: enum, values: [novice, intermediate, expert] }
```

This flat form lets serde derive directly.

- [ ] **Step 2: `src/lib.rs`**

```rust
//! YAML-driven schema loader.

pub mod error;
pub mod field;

pub use error::SchemaError;
pub use field::{FieldSpec, FieldType};

use kg_core::schema::{Cardinality, NodeSchema, PropType, RelSchema, SchemaRegistry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchemaFile {
    #[serde(default)]
    pub nodes: HashMap<String, NodeDef>,
    #[serde(default)]
    pub rels:  HashMap<String, RelDef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeDef {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub props: Vec<FieldSpec>,
    #[serde(default)]
    pub indexes: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelDef {
    #[serde(default)]
    pub endpoints: Vec<(String, String)>,
    #[serde(default = "default_cardinality")]
    pub cardinality: String,    // many_to_many | one_to_many | one_to_one
    #[serde(default)]
    pub props: Vec<FieldSpec>,
}

fn default_cardinality() -> String { "many_to_many".into() }

impl SchemaFile {
    pub fn from_yaml(path: &Path) -> Result<Self, SchemaError> {
        let raw = std::fs::read_to_string(path)?;
        let parsed: SchemaFile = serde_yaml::from_str(&raw)?;
        parsed.validate()?;
        Ok(parsed)
    }

    /// Cross-reference checks: every `ref` points to a known label; every rel
    /// endpoint pair references known labels.
    pub fn validate(&self) -> Result<(), SchemaError> {
        let labels: std::collections::HashSet<&str> = self.nodes.keys().map(String::as_str).collect();
        for (label, def) in &self.nodes {
            for spec in &def.props {
                if let FieldType::Ref { label: target } = &spec.ty {
                    if !labels.contains(target.as_str()) {
                        return Err(SchemaError::Invalid(format!(
                            "node `{label}` field `{}` references unknown label `{target}`",
                            spec.name
                        )));
                    }
                }
            }
        }
        for (ty, def) in &self.rels {
            for (s, e) in &def.endpoints {
                if !labels.contains(s.as_str()) {
                    return Err(SchemaError::Invalid(format!("rel `{ty}` endpoint start `{s}` not a declared node")));
                }
                if !labels.contains(e.as_str()) {
                    return Err(SchemaError::Invalid(format!("rel `{ty}` endpoint end `{e}` not a declared node")));
                }
            }
        }
        Ok(())
    }

    /// Project to the existing kg-core::SchemaRegistry. Enum + Ref collapse
    /// to String + Int respectively for core-level validation; the richer
    /// SchemaFile is kept alongside.
    pub fn to_core_registry(&self) -> SchemaRegistry {
        let mut reg = SchemaRegistry::new();
        for (label, def) in &self.nodes {
            let mut b = NodeSchema::builder(label);
            for spec in &def.props {
                let pt = match &spec.ty {
                    FieldType::String | FieldType::Date | FieldType::DateTime | FieldType::Enum { .. } => PropType::String,
                    FieldType::Int    | FieldType::Ref { .. } => PropType::Int,
                    FieldType::Float  => PropType::Float,
                    FieldType::Bool   => PropType::Bool,
                };
                b = b.prop(&spec.name, pt);
                if spec.required { b = b.required(); }
            }
            for ix in &def.indexes { b = b.index(ix.iter().cloned()); }
            reg.add_node(b.build());
        }
        for (ty, def) in &self.rels {
            let mut b = RelSchema::builder(ty);
            for (s, e) in &def.endpoints { b = b.endpoints(s, e); }
            let card = match def.cardinality.as_str() {
                "one_to_one"  => Cardinality::OneToOne,
                "one_to_many" => Cardinality::OneToMany,
                _             => Cardinality::ManyToMany,
            };
            b = b.cardinality(card);
            reg.add_rel(b.build());
        }
        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_yaml(s: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(s.as_bytes()).unwrap();
        f
    }

    #[test]
    fn parses_example_schema() {
        let f = write_yaml(include_str!("../../../kg-schema.yaml"));
        let s = SchemaFile::from_yaml(f.path()).unwrap();
        assert!(s.nodes.contains_key("Person"));
        assert!(s.rels.contains_key("KNOWS"));
    }

    #[test]
    fn rejects_ref_to_unknown_label() {
        let f = write_yaml(r#"
nodes:
  Foo:
    props:
      - { name: bar, type: ref, label: NotALabel }
"#);
        let e = SchemaFile::from_yaml(f.path()).unwrap_err();
        assert!(matches!(e, SchemaError::Invalid(_)));
    }

    #[test]
    fn rejects_rel_endpoint_to_unknown_label() {
        let f = write_yaml(r#"
nodes:
  Foo:
    props: []
rels:
  KNOWS:
    endpoints: [[Foo, Bar]]
"#);
        assert!(matches!(SchemaFile::from_yaml(f.path()).unwrap_err(), SchemaError::Invalid(_)));
    }

    #[test]
    fn projects_to_core_registry() {
        let f = write_yaml(include_str!("../../../kg-schema.yaml"));
        let s = SchemaFile::from_yaml(f.path()).unwrap();
        let reg = s.to_core_registry();
        assert!(reg.node("Person").is_some());
        assert!(reg.rel("KNOWS").is_some());
    }
}
```

- [ ] **Step 3: Test + commit**

```bash
cargo test -p kg-schema
git add Cargo.toml crates/kg-schema kg-schema.yaml
git commit -m "feat(kg-schema): SchemaFile YAML parser + validate + to_core_registry"
```

Expected: 4 tests pass.

---

## Task 5: kg-server — load SchemaFile at boot

**Files:**
- Modify: `crates/kg-server/Cargo.toml`
- Modify: `crates/kg-server/src/config.rs`
- Modify: `crates/kg-server/src/state.rs`
- Modify: `crates/kg-server/src/lib.rs`
- Modify: `crates/kg-server/src/bin/server.rs`

- [ ] **Step 1: Add kg-schema to deps**

```toml
[dependencies]
kg-schema = { path = "../kg-schema" }
# ... existing deps
```

- [ ] **Step 2: Update `config.rs`**

Add `schema_path` field:

```rust
pub struct Config {
    // ... existing fields
    pub schema_path: String,
}

impl Config {
    pub fn from_env() -> Self {
        // ... existing reads
        let schema_path = std::env::var("KG_SCHEMA")
            .expect("KG_SCHEMA env var required (path to kg-schema.yaml)");
        // construct as before plus schema_path
    }
}
```

- [ ] **Step 3: Update `state.rs`**

```rust
use std::sync::Arc;
use kg_neo4j::Client;
use kg_schema::SchemaFile;

#[derive(Clone)]
pub struct AppState {
    pub client: Arc<Client>,
    pub schema: Arc<SchemaFile>,
}
```

- [ ] **Step 4: Update `bin/server.rs`**

After loading config, load SchemaFile and pass into AppState:

```rust
let schema = SchemaFile::from_yaml(std::path::Path::new(&cfg.schema_path))
    .expect("failed to load KG_SCHEMA yaml");
let state = AppState {
    client: Arc::new(client),
    schema: Arc::new(schema),
};
```

- [ ] **Step 5: Build + commit**

```bash
cargo check -p kg-server
git add crates/kg-server
git commit -m "feat(kg-server): load SchemaFile from KG_SCHEMA env at boot"
```

---

## Task 6: kg-server — GET /schema endpoint

**Files:**
- Create: `crates/kg-server/src/routes/schema.rs`
- Modify: `crates/kg-server/src/routes/mod.rs`
- Modify: `crates/kg-server/src/lib.rs`

- [ ] **Step 1: `routes/schema.rs`**

```rust
use axum::{extract::State, Json};
use kg_schema::SchemaFile;
use crate::state::AppState;

pub async fn run(State(s): State<AppState>) -> Json<SchemaFile> {
    Json((*s.schema).clone())
}
```

- [ ] **Step 2: Register in mod.rs + lib.rs router**

```rust
// routes/mod.rs
pub mod commit;
pub mod health;
pub mod query;
pub mod schema;
```

In `router()` add: `.route("/schema", get(routes::schema::run))`.

- [ ] **Step 3: Smoke test**

Append to `crates/kg-server/tests/server.rs`:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn schema_returns_yaml() {
    let (base, _c) = start_server().await;
    let r = reqwest::get(format!("{base}/schema")).await.unwrap();
    assert!(r.status().is_success());
    let v: serde_json::Value = r.json().await.unwrap();
    assert!(v["nodes"].get("Person").is_some(), "expected Person in returned schema");
}
```

Note: `start_server` will need to also load a schema file. Update `start_server` (in tests/server.rs) to write a temp YAML file from `include_str!("../../../kg-schema.yaml")` and set the schema field on AppState.

- [ ] **Step 4: Run + commit**

```bash
cargo test -p kg-server -- --test-threads=1
git add crates/kg-server
git commit -m "feat(kg-server): GET /schema endpoint returning SchemaFile JSON"
```

---

## Task 7: kg-server — POST /entities (create node)

**Files:**
- Create: `crates/kg-server/src/routes/entities.rs`
- Modify: `crates/kg-server/src/lib.rs` (route registration)

- [ ] **Step 1: `routes/entities.rs`**

```rust
use axum::{extract::State, Json};
use kg_core::cypher::Statement;
use kg_core::value::PropValue;
use kg_schema::{FieldSpec, FieldType, SchemaFile};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateBody {
    pub label: String,
    pub props: BTreeMap<String, serde_json::Value>,
}

#[derive(Serialize)]
pub struct CreateResponse {
    pub id: i64,
    pub label: String,
    pub props: BTreeMap<String, serde_json::Value>,
}

pub async fn create(
    State(s): State<AppState>,
    Json(body): Json<CreateBody>,
) -> Result<Json<CreateResponse>, ApiError> {
    // 1. Validate against SchemaFile
    let node_def = s.schema.nodes.get(&body.label).ok_or_else(|| ApiError {
        code: "400.unknown_label".into(),
        message: format!("unknown node label `{}`", body.label),
    })?;
    validate_props(&node_def.props, &body.props)?;

    // 2. Build prop map for Cypher (JSON -> PropValue)
    let mut props: BTreeMap<String, PropValue> = BTreeMap::new();
    for (k, v) in &body.props {
        props.insert(k.clone(), json_to_prop(v.clone())?);
    }

    // 3. CREATE node + RETURN id, properties
    let stmt = Statement::new(
        format!("CREATE (n:`{}`) SET n = $props RETURN id(n) AS id, properties(n) AS props", body.label),
        [("props", PropValue::Map(props))],
    );
    let rows = s.client.query::<BTreeMap<String, PropValue>>(&stmt.cypher, stmt.params.into_iter().collect::<Vec<_>>())
        .await.map_err(ApiError::from)?;
    let row = rows.into_iter().next().ok_or_else(|| ApiError {
        code: "500.no_row".into(), message: "CREATE returned no row".into(),
    })?;
    let id = match row.get("id") {
        Some(PropValue::Int(i)) => *i,
        _ => return Err(ApiError { code: "500.bad_id".into(), message: "no id".into() }),
    };
    let props_json = match row.get("props") {
        Some(PropValue::Map(m)) => m.iter().map(|(k, v)| (k.clone(), prop_to_json(v))).collect(),
        _ => BTreeMap::new(),
    };
    Ok(Json(CreateResponse { id, label: body.label, props: props_json }))
}

fn validate_props(specs: &[FieldSpec], props: &BTreeMap<String, serde_json::Value>) -> Result<(), ApiError> {
    let mut errors = vec![];
    for spec in specs {
        match props.get(&spec.name) {
            None if spec.required => {
                errors.push(format!("missing required field `{}`", spec.name));
            }
            Some(v) if !type_matches(v, &spec.ty) => {
                errors.push(format!("field `{}` expected {:?}, got {v}", spec.name, spec.ty));
            }
            _ => {}
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ApiError { code: "400.validation".into(), message: errors.join("; ") })
    }
}

fn type_matches(v: &serde_json::Value, ty: &FieldType) -> bool {
    use serde_json::Value as J;
    match (v, ty) {
        (J::Null, _) => true,
        (J::Bool(_), FieldType::Bool) => true,
        (J::Number(n), FieldType::Int) => n.is_i64() || n.is_u64(),
        (J::Number(_), FieldType::Float) => true,
        (J::String(_), FieldType::String | FieldType::Date | FieldType::DateTime) => true,
        (J::String(s), FieldType::Enum { values }) => values.iter().any(|x| x == s),
        (J::Number(_), FieldType::Ref { .. }) => true,
        _ => false,
    }
}

fn json_to_prop(v: serde_json::Value) -> Result<PropValue, ApiError> {
    use serde_json::Value as J;
    Ok(match v {
        J::Null      => PropValue::Null,
        J::Bool(b)   => PropValue::Bool(b),
        J::Number(n) => {
            if let Some(i) = n.as_i64() { PropValue::Int(i) }
            else if let Some(f) = n.as_f64() { PropValue::Float(f) }
            else { return Err(ApiError { code: "400.bad_num".into(), message: format!("{n}") }); }
        }
        J::String(s) => PropValue::String(s),
        J::Array(a)  => PropValue::List(a.into_iter().map(json_to_prop).collect::<Result<_, _>>()?),
        J::Object(o) => {
            let mut m = BTreeMap::new();
            for (k, x) in o { m.insert(k, json_to_prop(x)?); }
            PropValue::Map(m)
        }
    })
}

fn prop_to_json(v: &PropValue) -> serde_json::Value {
    use PropValue::*;
    match v {
        Null => serde_json::Value::Null,
        Bool(b) => serde_json::Value::Bool(*b),
        Int(i) => serde_json::json!(i),
        Float(f) => serde_json::json!(f),
        String(s) => serde_json::Value::String(s.clone()),
        Bytes(_) => serde_json::Value::Null,
        List(xs) => serde_json::Value::Array(xs.iter().map(prop_to_json).collect()),
        Map(m) => serde_json::Value::Object(m.iter().map(|(k, v)| (k.clone(), prop_to_json(v))).collect()),
        Date(d) => serde_json::Value::String(d.format("%Y-%m-%d").to_string()),
        DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
        Duration { seconds, nanos } => serde_json::json!({"seconds": seconds, "nanos": nanos}),
        Point2D { srid, x, y } => serde_json::json!({"srid": srid, "x": x, "y": y}),
    }
}
```

- [ ] **Step 2: Wire in `lib.rs`**

```rust
pub mod routes {
    // ... existing
    pub mod entities;
}
// in router(): .route("/entities", post(routes::entities::create))
```

- [ ] **Step 3: Integration test**

Append to `crates/kg-server/tests/server.rs`:

```rust
#[tokio::test(flavor = "multi_thread")]
async fn create_entity_validates_and_persists() {
    let (base, _c) = start_server().await;
    let cli = reqwest::Client::new();

    // happy path
    let body = serde_json::json!({
        "label": "Person",
        "props": { "name": "Alice", "role": "engineer" }
    });
    let r = cli.post(format!("{base}/entities")).json(&body).send().await.unwrap();
    assert!(r.status().is_success(), "got {}: {}", r.status(), r.text().await.unwrap());
    let v: serde_json::Value = r.json().await.unwrap();
    assert_eq!(v["label"], "Person");
    assert!(v["id"].as_i64().is_some());

    // missing required `name`
    let body = serde_json::json!({"label": "Person", "props": { "role": "engineer" }});
    let r = cli.post(format!("{base}/entities")).json(&body).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 400);

    // wrong enum value
    let body = serde_json::json!({"label": "Person", "props": { "name": "Bob", "role": "wizard" }});
    let r = cli.post(format!("{base}/entities")).json(&body).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 400);
}
```

- [ ] **Step 4: Run + commit**

```bash
cargo test -p kg-server -- --test-threads=1
git add crates/kg-server
git commit -m "feat(kg-server): POST /entities with schema validation"
```

---

## Task 8: kg-server — PUT /entities/:id (patch node)

**Files:**
- Modify: `crates/kg-server/src/routes/entities.rs`

- [ ] **Step 1: Add update handler**

```rust
use axum::extract::Path;

#[derive(Deserialize)]
pub struct UpdateBody {
    pub set:   BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub unset: Vec<String>,
}

pub async fn update(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validate `set` against schema. We need to know the entity's label first.
    let labels_rows = s.client.query::<BTreeMap<String, PropValue>>(
        "MATCH (n) WHERE id(n) = $id RETURN labels(n) AS labels",
        [("id", PropValue::Int(id))],
    ).await.map_err(ApiError::from)?;
    let row = labels_rows.into_iter().next().ok_or_else(|| ApiError {
        code: "404.not_found".into(), message: format!("no node with id {id}"),
    })?;
    let label = match row.get("labels") {
        Some(PropValue::List(xs)) => xs.iter().find_map(|x| match x {
            PropValue::String(s) => Some(s.clone()), _ => None,
        }),
        _ => None,
    };
    if let Some(label) = label.as_deref() {
        if let Some(def) = s.schema.nodes.get(label) {
            let mut errs = vec![];
            for (k, v) in &body.set {
                if let Some(spec) = def.props.iter().find(|p| p.name == *k) {
                    if !type_matches(v, &spec.ty) {
                        errs.push(format!("field `{}` wrong type", k));
                    }
                }
            }
            if !errs.is_empty() {
                return Err(ApiError { code: "400.validation".into(), message: errs.join("; ") });
            }
        }
    }

    let mut sets = String::new();
    let mut params: Vec<(String, PropValue)> = vec![("id".into(), PropValue::Int(id))];
    let mut i = 0;
    for (k, v) in &body.set {
        let pkey = format!("p_{i}");
        sets.push_str(&format!(" SET n.`{k}` = ${pkey}"));
        params.push((pkey, json_to_prop(v.clone())?));
        i += 1;
    }
    for k in &body.unset {
        sets.push_str(&format!(" REMOVE n.`{k}`"));
    }
    let cypher = format!("MATCH (n) WHERE id(n) = $id{sets} RETURN id(n) AS id");
    let _ = s.client.query::<BTreeMap<String, PropValue>>(&cypher, params).await.map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}
```

- [ ] **Step 2: Register route**

In `lib.rs router()` add: `.route("/entities/:id", put(routes::entities::update))`.

- [ ] **Step 3: Test**

```rust
#[tokio::test(flavor = "multi_thread")]
async fn update_entity_set_and_unset() {
    let (base, _c) = start_server().await;
    let cli = reqwest::Client::new();
    let r = cli.post(format!("{base}/entities")).json(&serde_json::json!({
        "label": "Person", "props": {"name": "Eve", "role": "engineer", "age": 30}
    })).send().await.unwrap();
    let v: serde_json::Value = r.json().await.unwrap();
    let id = v["id"].as_i64().unwrap();

    let r = cli.put(format!("{base}/entities/{id}")).json(&serde_json::json!({
        "set": {"age": 31}, "unset": ["role"]
    })).send().await.unwrap();
    assert!(r.status().is_success());
}
```

- [ ] **Step 4: Commit**

```bash
cargo test -p kg-server -- --test-threads=1
git add crates/kg-server
git commit -m "feat(kg-server): PUT /entities/:id with schema validation on set"
```

---

## Task 9: kg-server — DELETE /entities/:id + GET /entities/:id

**Files:**
- Modify: `crates/kg-server/src/routes/entities.rs`

- [ ] **Step 1: Add delete + get_one**

```rust
use axum::extract::Query;

#[derive(Deserialize)]
pub struct DeleteParams {
    #[serde(default)]
    pub cascade: bool,
}

pub async fn delete(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    Query(p): Query<DeleteParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let cypher = if p.cascade {
        "MATCH (n) WHERE id(n) = $id DETACH DELETE n"
    } else {
        "MATCH (n) WHERE id(n) = $id DELETE n"
    };
    s.client.query::<BTreeMap<String, PropValue>>(cypher, [("id", PropValue::Int(id))]).await.map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn get_one(
    State(s): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = s.client.query::<BTreeMap<String, PropValue>>(
        "MATCH (n) WHERE id(n) = $id \
         OPTIONAL MATCH (n)-[r_out]->(t) \
         OPTIONAL MATCH (s)-[r_in]->(n) \
         RETURN id(n) AS id, labels(n) AS labels, properties(n) AS props, \
                collect(DISTINCT {id: id(r_out), type: type(r_out), target_id: id(t), target_labels: labels(t), props: properties(r_out)}) AS out_rels, \
                collect(DISTINCT {id: id(r_in), type: type(r_in), source_id: id(s), source_labels: labels(s), props: properties(r_in)}) AS in_rels",
        [("id", PropValue::Int(id))],
    ).await.map_err(ApiError::from)?;
    let row = rows.into_iter().next().ok_or_else(|| ApiError {
        code: "404.not_found".into(), message: format!("no node with id {id}"),
    })?;
    let body = serde_json::json!({
        "id": id,
        "labels": prop_to_json(row.get("labels").unwrap_or(&PropValue::Null)),
        "props": prop_to_json(row.get("props").unwrap_or(&PropValue::Null)),
        "out_rels": prop_to_json(row.get("out_rels").unwrap_or(&PropValue::Null)),
        "in_rels": prop_to_json(row.get("in_rels").unwrap_or(&PropValue::Null)),
    });
    Ok(Json(body))
}
```

- [ ] **Step 2: Register**

```rust
// in router():
.route("/entities/:id", get(routes::entities::get_one).put(routes::entities::update).delete(routes::entities::delete))
```

- [ ] **Step 3: Test + commit**

Add one integration test for the cascade delete path. Commit:

```bash
git commit -m "feat(kg-server): DELETE /entities/:id with cascade; GET /entities/:id"
```

---

## Task 10: kg-server — Links + List + Search

**Files:**
- Create: `crates/kg-server/src/routes/links.rs`
- Create: `crates/kg-server/src/routes/search.rs`
- Modify: `crates/kg-server/src/routes/entities.rs` (add `list`)

- [ ] **Step 1: `routes/entities.rs::list`**

```rust
#[derive(Deserialize)]
pub struct ListParams {
    pub label: Option<String>,
    pub q: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}
fn default_limit() -> i64 { 50 }

pub async fn list(
    State(s): State<AppState>,
    Query(p): Query<ListParams>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    // Simple LIKE-style search: matches against any string property if q given.
    let (cypher, params) = match (p.label.as_deref(), p.q.as_deref()) {
        (Some(label), Some(q)) => (
            format!("MATCH (n:`{label}`) WHERE any(k IN keys(n) WHERE toLower(toString(n[k])) CONTAINS toLower($q)) RETURN id(n) AS id, labels(n) AS labels, properties(n) AS props LIMIT $lim"),
            vec![("q".into(), PropValue::String(q.into())), ("lim".into(), PropValue::Int(p.limit))],
        ),
        (Some(label), None) => (
            format!("MATCH (n:`{label}`) RETURN id(n) AS id, labels(n) AS labels, properties(n) AS props LIMIT $lim"),
            vec![("lim".into(), PropValue::Int(p.limit))],
        ),
        (None, Some(q)) => (
            "MATCH (n) WHERE any(k IN keys(n) WHERE toLower(toString(n[k])) CONTAINS toLower($q)) RETURN id(n) AS id, labels(n) AS labels, properties(n) AS props LIMIT $lim".into(),
            vec![("q".into(), PropValue::String(q.into())), ("lim".into(), PropValue::Int(p.limit))],
        ),
        (None, None) => (
            "MATCH (n) RETURN id(n) AS id, labels(n) AS labels, properties(n) AS props LIMIT $lim".into(),
            vec![("lim".into(), PropValue::Int(p.limit))],
        ),
    };
    let rows = s.client.query::<BTreeMap<String, PropValue>>(&cypher, params).await.map_err(ApiError::from)?;
    Ok(Json(rows.into_iter().map(|r| serde_json::json!({
        "id":     prop_to_json(r.get("id").unwrap_or(&PropValue::Null)),
        "labels": prop_to_json(r.get("labels").unwrap_or(&PropValue::Null)),
        "props":  prop_to_json(r.get("props").unwrap_or(&PropValue::Null)),
    })).collect()))
}
```

- [ ] **Step 2: `routes/links.rs`**

```rust
use axum::{extract::{Path, State}, Json};
use kg_core::value::PropValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use crate::error::ApiError;
use crate::routes::entities::{json_to_prop, prop_to_json};   // make those pub(crate)
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateLink {
    pub r#type: String,
    pub start_id: i64,
    pub end_id: i64,
    #[serde(default)]
    pub props: BTreeMap<String, serde_json::Value>,
}

#[derive(Serialize)]
pub struct CreateLinkResponse {
    pub id: i64,
}

pub async fn create(
    State(s): State<AppState>,
    Json(body): Json<CreateLink>,
) -> Result<Json<CreateLinkResponse>, ApiError> {
    // Validate endpoint labels against rel schema.
    if let Some(rel_def) = s.schema.rels.get(&body.r#type) {
        if !rel_def.endpoints.is_empty() {
            let label_rows = s.client.query::<BTreeMap<String, PropValue>>(
                "MATCH (s),(e) WHERE id(s) = $sid AND id(e) = $eid RETURN labels(s)[0] AS sl, labels(e)[0] AS el",
                [("sid", PropValue::Int(body.start_id)), ("eid", PropValue::Int(body.end_id))],
            ).await.map_err(ApiError::from)?;
            if let Some(row) = label_rows.into_iter().next() {
                let sl = if let Some(PropValue::String(s)) = row.get("sl") { Some(s.clone()) } else { None };
                let el = if let Some(PropValue::String(s)) = row.get("el") { Some(s.clone()) } else { None };
                if let (Some(s), Some(e)) = (sl, el) {
                    if !rel_def.endpoints.iter().any(|(a, b)| a == &s && b == &e) {
                        return Err(ApiError {
                            code: "400.endpoint_not_allowed".into(),
                            message: format!("rel `{}` not allowed from `{s}` to `{e}`", body.r#type),
                        });
                    }
                }
            }
        }
    }

    let mut props: BTreeMap<String, PropValue> = BTreeMap::new();
    for (k, v) in body.props { props.insert(k, json_to_prop(v)?); }
    let rows = s.client.query::<BTreeMap<String, PropValue>>(
        &format!("MATCH (s),(e) WHERE id(s) = $sid AND id(e) = $eid CREATE (s)-[r:`{}` $p]->(e) RETURN id(r) AS id", body.r#type),
        vec![
            ("sid".into(), PropValue::Int(body.start_id)),
            ("eid".into(), PropValue::Int(body.end_id)),
            ("p".into(),   PropValue::Map(props)),
        ],
    ).await.map_err(ApiError::from)?;
    let row = rows.into_iter().next().ok_or_else(|| ApiError {
        code: "500.no_row".into(), message: "no row".into(),
    })?;
    let id = match row.get("id") {
        Some(PropValue::Int(i)) => *i,
        _ => return Err(ApiError { code: "500.bad_id".into(), message: "no id".into() }),
    };
    Ok(Json(CreateLinkResponse { id }))
}

pub async fn delete(
    State(s): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    s.client.query::<BTreeMap<String, PropValue>>(
        "MATCH ()-[r]->() WHERE id(r) = $id DELETE r",
        [("id", PropValue::Int(id))],
    ).await.map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}
```

- [ ] **Step 3: `routes/search.rs`**

Phase-2 minimum: alias for `list` with no label filter.

```rust
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::error::ApiError;
use crate::routes::entities::{list, ListParams};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
}
fn default_limit() -> i64 { 20 }

pub async fn run(
    s: State<AppState>,
    Query(p): Query<SearchParams>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    list(s, Query(ListParams { label: None, q: Some(p.q), limit: p.limit })).await
}
```

(Make `entities::list` and `entities::ListParams` `pub(crate)` so search.rs can import.)

- [ ] **Step 4: Register routes + tests**

```rust
// router():
.route("/entities", get(routes::entities::list).post(routes::entities::create))
.route("/entities/:id", get(routes::entities::get_one).put(routes::entities::update).delete(routes::entities::delete))
.route("/links", post(routes::links::create))
.route("/links/:id", delete(routes::links::delete))
.route("/search", get(routes::search::run))
```

Add one integration test per endpoint (list, create_link, delete_link, search). Smoke shape only.

- [ ] **Step 5: Commit**

```bash
cargo test -p kg-server -- --test-threads=1
git add crates/kg-server
git commit -m "feat(kg-server): /entities (list,get,create,update,delete), /links, /search"
```

---

## Task 11: kg-mcp crate scaffold

**Files:**
- Create: `crates/kg-mcp/Cargo.toml`
- Create: `crates/kg-mcp/src/main.rs`
- Create: `crates/kg-mcp/src/server.rs`
- Create: `crates/kg-mcp/src/client.rs`
- Create: `crates/kg-mcp/src/tools.rs`

- [ ] **Step 1: Pick an MCP crate**

Inspect crates.io for the official MCP server crate. As of 2026, `rmcp` (or
similar) is the leading Rust implementation. Use its `ServerHandler` trait
and JSON-RPC stdio transport. If the official crate has a different name,
adapt — the structure of the implementation is the same.

- [ ] **Step 2: `Cargo.toml`**

```toml
[package]
name = "kg-mcp"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "MCP stdio server that exposes kg-server as Claude-callable tools."

[dependencies]
serde      = { workspace = true }
serde_json = { workspace = true }
tokio      = { workspace = true, features = ["macros", "rt-multi-thread", "io-std"] }
tracing    = { workspace = true }
tracing-subscriber = { workspace = true }
reqwest    = { workspace = true }
anyhow     = "1"
rmcp       = "0.1"     # adjust to actual published version
```

- [ ] **Step 3: `src/main.rs`**

Minimal scaffold. Real wiring lands in Task 12/13.

```rust
mod client;
mod server;
mod tools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").with_writer(std::io::stderr).init();
    server::run().await
}
```

- [ ] **Step 4: Stubs**

`src/server.rs`:
```rust
pub async fn run() -> anyhow::Result<()> {
    // wire MCP stdio transport here in T12
    Ok(())
}
```

`src/client.rs`:
```rust
//! Thin wrapper around kg-server REST.

pub struct KgClient {
    pub base: String,
    pub http: reqwest::Client,
}

impl KgClient {
    pub fn from_env() -> Self {
        let base = std::env::var("KG_SERVER_URL").unwrap_or_else(|_| "http://localhost:9000".into());
        Self { base, http: reqwest::Client::new() }
    }
}
```

`src/tools.rs`:
```rust
//! Tool implementations (created in T12).
```

- [ ] **Step 5: Build + commit**

```bash
cargo check -p kg-mcp
git add crates/kg-mcp
git commit -m "feat(kg-mcp): scaffold MCP stdio server"
```

---

## Task 12: kg-mcp — tool implementations

**Files:**
- Modify: `crates/kg-mcp/src/client.rs`
- Modify: `crates/kg-mcp/src/tools.rs`

- [ ] **Step 1: Expand KgClient**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct CreateEntityArgs {
    pub label: String,
    #[serde(default)]
    pub props: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct EntityResponse {
    pub id: i64,
    pub label: String,
    pub props: serde_json::Value,
}

impl KgClient {
    pub async fn get_schema(&self) -> anyhow::Result<serde_json::Value> {
        Ok(self.http.get(format!("{}/schema", self.base)).send().await?.error_for_status()?.json().await?)
    }
    pub async fn create_entity(&self, args: &CreateEntityArgs) -> anyhow::Result<EntityResponse> {
        Ok(self.http.post(format!("{}/entities", self.base))
            .json(args)
            .send().await?
            .error_for_status()?
            .json().await?)
    }
    pub async fn update_entity(&self, id: i64, set: &serde_json::Map<String, serde_json::Value>, unset: &[String]) -> anyhow::Result<()> {
        self.http.put(format!("{}/entities/{}", self.base, id))
            .json(&serde_json::json!({"set": set, "unset": unset}))
            .send().await?
            .error_for_status()?;
        Ok(())
    }
    pub async fn delete_entity(&self, id: i64, cascade: bool) -> anyhow::Result<()> {
        self.http.delete(format!("{}/entities/{}?cascade={}", self.base, id, cascade))
            .send().await?
            .error_for_status()?;
        Ok(())
    }
    pub async fn link_entities(&self, ty: &str, start_id: i64, end_id: i64, props: &serde_json::Map<String, serde_json::Value>) -> anyhow::Result<i64> {
        let v: serde_json::Value = self.http.post(format!("{}/links", self.base))
            .json(&serde_json::json!({"type": ty, "start_id": start_id, "end_id": end_id, "props": props}))
            .send().await?
            .error_for_status()?
            .json().await?;
        Ok(v["id"].as_i64().ok_or_else(|| anyhow::anyhow!("no id"))?)
    }
    pub async fn unlink_entities(&self, rel_id: i64) -> anyhow::Result<()> {
        self.http.delete(format!("{}/links/{}", self.base, rel_id))
            .send().await?
            .error_for_status()?;
        Ok(())
    }
    pub async fn find_entities(&self, label: Option<&str>, q: Option<&str>, limit: i64) -> anyhow::Result<serde_json::Value> {
        let mut url = reqwest::Url::parse(&format!("{}/entities", self.base))?;
        if let Some(l) = label { url.query_pairs_mut().append_pair("label", l); }
        if let Some(qq) = q { url.query_pairs_mut().append_pair("q", qq); }
        url.query_pairs_mut().append_pair("limit", &limit.to_string());
        Ok(self.http.get(url).send().await?.error_for_status()?.json().await?)
    }
}
```

- [ ] **Step 2: `src/tools.rs`** — wire each tool

Use the rmcp tool registration API. Pseudocode shape:

```rust
use rmcp::{Tool, ToolInput, ToolResult};
use crate::client::{KgClient, CreateEntityArgs};

pub fn register_tools(srv: &mut rmcp::Server, kg: KgClient) {
    srv.add_tool(Tool::new("get_schema", "Get the schema (labels, props, rels).",
        |_: ()| async { kg.get_schema().await }
    ));

    srv.add_tool(Tool::new("create_entity", "Create a node with the given label and props.",
        |args: CreateEntityArgs| async { kg.create_entity(&args).await }
    ));

    // ... and the rest
}
```

Adapt to the actual rmcp API. The key requirement: each tool is registered
with a stable name, a one-line description, and an async handler that
returns serde_json::Value (or a strongly-typed response).

- [ ] **Step 3: Wire `server::run`**

```rust
use rmcp::transports::StdioTransport;
use crate::client::KgClient;

pub async fn run() -> anyhow::Result<()> {
    let kg = KgClient::from_env();
    let mut server = rmcp::Server::new("kg-mcp", "0.1.0");
    crate::tools::register_tools(&mut server, kg);
    server.run(StdioTransport).await?;
    Ok(())
}
```

- [ ] **Step 4: Smoke build**

```bash
cargo check -p kg-mcp
```

- [ ] **Step 5: Commit**

```bash
git add crates/kg-mcp
git commit -m "feat(kg-mcp): 7 tools (create/update/delete/link/unlink/find/get_schema)"
```

---

## Task 13: kg-mcp — subprocess test

**Files:**
- Create: `crates/kg-mcp/tests/stdio.rs`

- [ ] **Step 1: Test scaffold**

This test spawns the kg-mcp binary, points it at a mock HTTP server (built
with `tokio::net::TcpListener` + `axum`), and exercises one tool call over
stdio.

```rust
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

#[tokio::test(flavor = "multi_thread")]
async fn create_entity_tool_works() {
    // Start a tiny mock HTTP server that responds to POST /entities.
    let mock = tokio::spawn(async move {
        let app = axum::Router::new().route("/entities", axum::routing::post(|axum::Json(_v): axum::Json<serde_json::Value>| async {
            axum::Json(serde_json::json!({"id": 42, "label": "Person", "props": {}}))
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        addr
    }).await.unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_kg-mcp"))
        .env("KG_SERVER_URL", format!("http://{mock}"))
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit())
        .spawn().unwrap();

    let stdin = child.stdin.as_mut().unwrap();
    let stdout = child.stdout.as_mut().unwrap();
    let mut reader = BufReader::new(stdout);

    // Send a JSON-RPC initialize request, then a tool call.
    // (Exact format depends on the chosen MCP crate.)
    stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n").await.unwrap();
    stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"create_entity\",\"arguments\":{\"label\":\"Person\",\"props\":{\"name\":\"Alice\"}}}}\n").await.unwrap();
    stdin.flush().await.unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    assert!(line.contains("\"id\":1"));
    line.clear();
    reader.read_line(&mut line).await.unwrap();
    assert!(line.contains("\"id\":42") || line.contains("Alice"));

    child.kill().await.unwrap();
}
```

The exact wire format depends on the MCP crate. If rmcp has a higher-level
test helper, prefer that. Keep the goal stable: spawn the binary, send a
tool call, parse the response.

- [ ] **Step 2: Run + commit**

```bash
cargo test -p kg-mcp
git add crates/kg-mcp/tests
git commit -m "test(kg-mcp): subprocess test for create_entity tool"
```

---

## Task 14: webui — deps + strip phase-1 components

**Files:**
- Modify: `webui/package.json`
- Delete: `webui/src/components/{Canvas,Inspector,Toolbar,QueryBox,ResultsTable}.tsx`
- Delete: `webui/src/kg/{refresh,uow}.ts`

- [ ] **Step 1: Install new deps**

```bash
cd webui
npm install react-router-dom@^6.26 react-hook-form@^7.52 zod@^3.23 @tanstack/react-query@^5.51
npm install --save-dev @testing-library/react @testing-library/jest-dom @types/node
```

(shadcn vendoring happens in T26.)

- [ ] **Step 2: Delete the 5 phase-1 component files + 2 kg helpers**

These are replaced by phase-2 components. Don't bother stubbing — App.tsx
will reference them no longer.

- [ ] **Step 3: Build (will fail — that's OK)**

`npm run build` will fail because App.tsx imports the deleted modules. The
next task replaces App.tsx.

- [ ] **Step 4: Commit**

```bash
git add webui
git commit -m "chore(webui): install router/forms/query deps; remove phase-1 components"
```

---

## Task 15: webui — schema-aware types + useSchema hook

**Files:**
- Create: `webui/src/kg/schema.ts`
- Create: `webui/src/hooks/useSchema.ts`

- [ ] **Step 1: `webui/src/kg/schema.ts`**

Mirror the kg-schema `SchemaFile` shape in TS.

```ts
export type FieldType =
  | { type: "string" } | { type: "int" } | { type: "float" } | { type: "bool" }
  | { type: "date" } | { type: "date_time" }
  | { type: "enum"; values: string[] }
  | { type: "ref";  label: string };

export interface FieldSpec extends FieldType {
  name: string;
  required?: boolean;
  unique?: boolean;
  default?: unknown;
}

export interface NodeDef {
  description?: string;
  props: FieldSpec[];
  indexes?: string[][];
}
export interface RelDef {
  endpoints: [string, string][];
  cardinality: "many_to_many" | "one_to_many" | "one_to_one";
  props: FieldSpec[];
}
export interface SchemaFile {
  nodes: Record<string, NodeDef>;
  rels: Record<string, RelDef>;
}
```

Note: `FieldSpec extends FieldType` flattens `type` + variant fields onto the same object. This matches `#[serde(flatten)]` on the Rust side.

- [ ] **Step 2: `webui/src/hooks/useSchema.ts`**

```ts
import { useQuery } from "@tanstack/react-query";
import type { SchemaFile } from "../kg/schema";

const BASE = (import.meta.env.VITE_KG_SERVER_BASE as string | undefined) ?? "http://localhost:9000";

export function useSchema() {
  return useQuery<SchemaFile>({
    queryKey: ["schema"],
    queryFn: async () => {
      const r = await fetch(`${BASE}/schema`);
      if (!r.ok) throw new Error(`schema fetch ${r.status}`);
      return r.json();
    },
    staleTime: Infinity,   // schema is boot-loaded, never changes during a session
  });
}
```

- [ ] **Step 3: Commit**

```bash
git add webui/src/kg/schema.ts webui/src/hooks/useSchema.ts
git commit -m "feat(webui): SchemaFile TS types + useSchema hook"
```

---

## Task 16: webui — REST client rewrite

**Files:**
- Modify: `webui/src/kg/client.ts`

- [ ] **Step 1: Replace with REST endpoints from T6-T10**

```ts
const BASE = (import.meta.env.VITE_KG_SERVER_BASE as string | undefined) ?? "http://localhost:9000";

async function asJson<T>(r: Response): Promise<T> {
  if (!r.ok) {
    const t = await r.text();
    let body: unknown = t;
    try { body = JSON.parse(t); } catch {}
    throw new Error(`server ${r.status}: ${JSON.stringify(body)}`);
  }
  return r.json() as Promise<T>;
}

export interface EntityCreateBody { label: string; props: Record<string, unknown>; }
export interface EntityResponse  { id: number; label: string; props: Record<string, unknown>; }
export interface EntityDetail {
  id: number; labels: string[]; props: Record<string, unknown>;
  out_rels: Array<{ id: number; type: string; target_id: number; target_labels: string[]; props: Record<string, unknown> }>;
  in_rels:  Array<{ id: number; type: string; source_id: number; source_labels: string[]; props: Record<string, unknown> }>;
}

export async function listEntities(label?: string, q?: string, limit = 50): Promise<EntityDetail[]> {
  const u = new URL(`${BASE}/entities`);
  if (label) u.searchParams.set("label", label);
  if (q)     u.searchParams.set("q", q);
  u.searchParams.set("limit", String(limit));
  return asJson(await fetch(u));
}

export async function getEntity(id: number): Promise<EntityDetail> {
  return asJson(await fetch(`${BASE}/entities/${id}`));
}

export async function createEntity(body: EntityCreateBody): Promise<EntityResponse> {
  return asJson(await fetch(`${BASE}/entities`, {
    method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body),
  }));
}

export async function updateEntity(id: number, set: Record<string, unknown>, unset: string[] = []): Promise<void> {
  await asJson(await fetch(`${BASE}/entities/${id}`, {
    method: "PUT", headers: { "content-type": "application/json" }, body: JSON.stringify({ set, unset }),
  }));
}

export async function deleteEntity(id: number, cascade = true): Promise<void> {
  await asJson(await fetch(`${BASE}/entities/${id}?cascade=${cascade}`, { method: "DELETE" }));
}

export async function createLink(type: string, start_id: number, end_id: number, props: Record<string, unknown> = {}): Promise<{ id: number }> {
  return asJson(await fetch(`${BASE}/links`, {
    method: "POST", headers: { "content-type": "application/json" },
    body: JSON.stringify({ type, start_id, end_id, props }),
  }));
}

export async function deleteLink(id: number): Promise<void> {
  await asJson(await fetch(`${BASE}/links/${id}`, { method: "DELETE" }));
}

export async function search(q: string, limit = 20): Promise<EntityDetail[]> {
  const u = new URL(`${BASE}/search`);
  u.searchParams.set("q", q);
  u.searchParams.set("limit", String(limit));
  return asJson(await fetch(u));
}
```

- [ ] **Step 2: Commit**

```bash
git add webui/src/kg/client.ts
git commit -m "feat(webui): REST client for /entities, /links, /search"
```

---

## Task 17: webui — pending ops store

**Files:**
- Modify: `webui/src/state/store.ts`
- Create: `webui/src/kg/pending.ts`

- [ ] **Step 1: `webui/src/kg/pending.ts`**

```ts
import type { Props } from "../kg/types";

export type PendingOp =
  | { kind: "create_node"; tmpId: string; label: string; props: Props }
  | { kind: "update_node"; id: number; set: Props; unset: string[] }
  | { kind: "delete_node"; id: number; cascade: boolean }
  | { kind: "create_link"; tmpId: string; type: string; start: { kind: "id"|"tmp"; ref: string|number }; end: { kind: "id"|"tmp"; ref: string|number }; props: Props }
  | { kind: "delete_link"; id: number };

export function describe(op: PendingOp): string {
  switch (op.kind) {
    case "create_node": return `Create ${op.label} ${op.props.name ? `"${op.props.name}"` : ""}`;
    case "update_node": return `Update node #${op.id}`;
    case "delete_node": return `Delete node #${op.id}`;
    case "create_link": return `Link ${op.type} ${refSummary(op.start)} → ${refSummary(op.end)}`;
    case "delete_link": return `Delete link #${op.id}`;
  }
}
function refSummary(r: { kind: "id"|"tmp"; ref: string|number }): string {
  return r.kind === "id" ? `#${r.ref}` : `(new) ${r.ref}`;
}
```

- [ ] **Step 2: Replace `webui/src/state/store.ts`**

```ts
import { create } from "zustand";
import type { PendingOp } from "../kg/pending";

export interface Store {
  pending: PendingOp[];
  append(op: PendingOp): void;
  remove(idx: number): void;
  clear(): void;
}

export const useStore = create<Store>((set) => ({
  pending: [],
  append: (op) => set((s) => ({ pending: [...s.pending, op] })),
  remove: (idx) => set((s) => ({ pending: s.pending.filter((_, i) => i !== idx) })),
  clear: () => set({ pending: [] }),
}));
```

(Drop the phase-1 graph state; React Query owns server cache now.)

- [ ] **Step 3: Update Vitest test if it exists; or rewrite to match new store**

Replace `webui/src/tests/store.test.ts` with tests that exercise the new `append`/`remove`/`clear` actions.

- [ ] **Step 4: Commit**

```bash
git add webui/src/state webui/src/kg webui/src/tests
git commit -m "feat(webui): pending ops store + describe()"
```

---

## Task 18: webui — EntityForm component (schema-driven)

**Files:**
- Create: `webui/src/components/EntityForm.tsx`

- [ ] **Step 1: Implement**

```tsx
import { useMemo } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";    // npm install @hookform/resolvers
import type { FieldSpec } from "../kg/schema";

interface Props {
  fields: FieldSpec[];
  initial?: Record<string, unknown>;
  onSubmit: (values: Record<string, unknown>) => void;
  submitLabel?: string;
}

export default function EntityForm({ fields, initial, onSubmit, submitLabel = "Save" }: Props) {
  const schema = useMemo(() => buildZodSchema(fields), [fields]);
  const { register, handleSubmit, formState: { errors } } = useForm({
    resolver: zodResolver(schema),
    defaultValues: initial,
  });

  return (
    <form onSubmit={handleSubmit(onSubmit)} style={{ display: "grid", gap: 8 }}>
      {fields.map((f) => (
        <FieldInput key={f.name} field={f} register={register} error={errors[f.name]?.message as string | undefined} />
      ))}
      <button type="submit">{submitLabel}</button>
    </form>
  );
}

function buildZodSchema(fields: FieldSpec[]): z.ZodTypeAny {
  const shape: Record<string, z.ZodTypeAny> = {};
  for (const f of fields) {
    let s: z.ZodTypeAny;
    switch (f.type) {
      case "string": case "date": case "date_time": s = z.string(); break;
      case "int": s = z.coerce.number().int(); break;
      case "float": s = z.coerce.number(); break;
      case "bool": s = z.coerce.boolean(); break;
      case "enum": s = z.enum(f.values as [string, ...string[]]); break;
      case "ref": s = z.coerce.number().int(); break;
    }
    shape[f.name] = f.required ? s : s.optional();
  }
  return z.object(shape);
}

function FieldInput({ field, register, error }: { field: FieldSpec; register: ReturnType<typeof useForm>["register"]; error?: string }) {
  const id = `field-${field.name}`;
  let input: JSX.Element;
  switch (field.type) {
    case "string":
      input = <input id={id} {...register(field.name)} />;
      break;
    case "int":
    case "float":
    case "ref":
      input = <input id={id} type="number" {...register(field.name)} />;
      break;
    case "bool":
      input = <input id={id} type="checkbox" {...register(field.name)} />;
      break;
    case "date":
      input = <input id={id} type="date" {...register(field.name)} />;
      break;
    case "date_time":
      input = <input id={id} type="datetime-local" {...register(field.name)} />;
      break;
    case "enum":
      input = (
        <select id={id} {...register(field.name)}>
          <option value="">(unset)</option>
          {field.values.map((v) => <option key={v} value={v}>{v}</option>)}
        </select>
      );
      break;
  }
  return (
    <label htmlFor={id} style={{ display: "grid", gridTemplateColumns: "140px 1fr", gap: 8, alignItems: "center" }}>
      <span>{field.name}{field.required ? " *" : ""}</span>
      <div>
        {input}
        {error && <div style={{ color: "red", fontSize: 12 }}>{error}</div>}
      </div>
    </label>
  );
}
```

Install missing dep:
```bash
cd webui && npm install @hookform/resolvers
```

- [ ] **Step 2: Commit**

```bash
cd webui && npm run build
git add webui
git commit -m "feat(webui): schema-driven EntityForm using react-hook-form + zod"
```

---

## Task 19: webui — EntityList + EntityListSidebar

**Files:**
- Create: `webui/src/components/EntityList.tsx`
- Create: `webui/src/components/EntityListSidebar.tsx`
- Create: `webui/src/hooks/useEntities.ts`

- [ ] **Step 1: `useEntities` hook**

```ts
import { useQuery } from "@tanstack/react-query";
import { listEntities, type EntityDetail } from "../kg/client";

export function useEntities(label?: string, q?: string, limit = 50) {
  return useQuery<EntityDetail[]>({
    queryKey: ["entities", label, q, limit],
    queryFn: () => listEntities(label, q, limit),
  });
}
```

- [ ] **Step 2: `EntityList`**

```tsx
import { Link } from "react-router-dom";
import { useEntities } from "../hooks/useEntities";

export default function EntityList({ label, q }: { label?: string; q?: string }) {
  const { data, isLoading, error } = useEntities(label, q);
  if (isLoading) return <p>Loading…</p>;
  if (error) return <p style={{ color: "red" }}>{String(error)}</p>;
  if (!data || data.length === 0) return <p>No entities.</p>;

  // Show the most distinctive prop as the "name" column. Fall back to props[0].
  const sampleProps = data[0]?.props ?? {};
  const nameKey = "name" in sampleProps ? "name" : Object.keys(sampleProps)[0] ?? "id";
  const otherKeys = Object.keys(sampleProps).filter((k) => k !== nameKey).slice(0, 3);

  return (
    <table style={{ width: "100%", borderCollapse: "collapse" }}>
      <thead>
        <tr><th>{nameKey}</th>{otherKeys.map((k) => <th key={k}>{k}</th>)}<th>id</th></tr>
      </thead>
      <tbody>
        {data.map((e) => (
          <tr key={e.id}>
            <td><Link to={`/entity/${e.id}`}>{String(e.props[nameKey] ?? "")}</Link></td>
            {otherKeys.map((k) => <td key={k}>{String(e.props[k] ?? "")}</td>)}
            <td>{e.id}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
```

- [ ] **Step 3: `EntityListSidebar`**

```tsx
import { Link, useParams } from "react-router-dom";
import { useSchema } from "../hooks/useSchema";
import { useEntities } from "../hooks/useEntities";

export default function EntityListSidebar() {
  const { data: schema } = useSchema();
  if (!schema) return null;
  const labels = Object.keys(schema.nodes).sort();
  return (
    <aside style={{ width: 240, borderRight: "1px solid #d1d5db", padding: 12, overflow: "auto" }}>
      {labels.map((label) => <LabelGroup key={label} label={label} />)}
    </aside>
  );
}

function LabelGroup({ label }: { label: string }) {
  const { data } = useEntities(label);
  const { id: currentId } = useParams();
  return (
    <details open style={{ marginBottom: 8 }}>
      <summary><strong>{label}</strong> ({data?.length ?? "…"})</summary>
      <ul style={{ paddingLeft: 16, listStyle: "none" }}>
        {data?.slice(0, 25).map((e) => (
          <li key={e.id}>
            <Link to={`/entity/${e.id}`} style={{ fontWeight: String(e.id) === currentId ? "bold" : "normal" }}>
              {String(e.props.name ?? e.id)}
            </Link>
          </li>
        ))}
      </ul>
      <Link to={`/browse?label=${label}&add=1`}>+ Add {label}</Link>
    </details>
  );
}
```

- [ ] **Step 4: Commit**

```bash
git add webui/src
git commit -m "feat(webui): EntityList + sidebar with per-label groups"
```

---

## Task 20: webui — Pending panel

**Files:**
- Create: `webui/src/components/PendingPanel.tsx`

- [ ] **Step 1: Implement**

```tsx
import { useState } from "react";
import { useStore } from "../state/store";
import { describe, type PendingOp } from "../kg/pending";
import * as api from "../kg/client";

export default function PendingPanel({ onCommitted }: { onCommitted?: () => void }) {
  const { pending, remove, clear } = useStore();
  const [running, setRunning] = useState(false);

  if (pending.length === 0) return null;

  const commit = async () => {
    setRunning(true);
    const tmpToId: Record<string, number> = {};
    try {
      for (const op of pending) {
        await applyOp(op, tmpToId);
      }
      clear();
      onCommitted?.();
    } catch (e) {
      alert(String(e));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div style={{ position: "fixed", left: 0, right: 0, bottom: 0, background: "#fef3c7", borderTop: "1px solid #d97706", padding: 12 }}>
      <strong>{pending.length} pending changes</strong>
      <ul style={{ margin: "8px 0", paddingLeft: 18 }}>
        {pending.map((op, i) => (
          <li key={i}>
            {describe(op)}{" "}
            <button onClick={() => remove(i)} disabled={running}>×</button>
          </li>
        ))}
      </ul>
      <button onClick={() => clear()} disabled={running}>Discard all</button>{" "}
      <button onClick={commit} disabled={running}>{running ? "Committing…" : "Commit all"}</button>
    </div>
  );
}

async function applyOp(op: PendingOp, tmpToId: Record<string, number>): Promise<void> {
  switch (op.kind) {
    case "create_node": {
      const r = await api.createEntity({ label: op.label, props: op.props });
      tmpToId[op.tmpId] = r.id;
      break;
    }
    case "update_node": await api.updateEntity(op.id, op.set, op.unset); break;
    case "delete_node": await api.deleteEntity(op.id, op.cascade); break;
    case "create_link": {
      const sid = op.start.kind === "id" ? (op.start.ref as number) : tmpToId[op.start.ref as string];
      const eid = op.end.kind   === "id" ? (op.end.ref   as number) : tmpToId[op.end.ref   as string];
      if (sid == null || eid == null) throw new Error("unresolved tmpId in link");
      await api.createLink(op.type, sid, eid, op.props);
      break;
    }
    case "delete_link": await api.deleteLink(op.id); break;
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add webui/src/components/PendingPanel.tsx
git commit -m "feat(webui): PendingPanel with per-op discard and commit-all"
```

---

## Task 21: webui — RelationshipPicker

**Files:**
- Create: `webui/src/components/RelationshipPicker.tsx`

- [ ] **Step 1: Implement**

```tsx
import { useState } from "react";
import { useSchema } from "../hooks/useSchema";
import { useEntities } from "../hooks/useEntities";
import EntityForm from "./EntityForm";
import type { Props } from "../kg/types";

interface PickerProps {
  fromId: number;
  fromLabel: string;
  onStage: (type: string, targetId: number, props: Props) => void;
  onClose: () => void;
}

export default function RelationshipPicker(p: PickerProps) {
  const { data: schema } = useSchema();
  const [type, setType] = useState<string | null>(null);
  const [target, setTarget] = useState<{ id: number; label: string } | null>(null);

  if (!schema) return null;
  const allowedTypes = Object.entries(schema.rels).filter(([, def]) =>
    def.endpoints.some(([s]) => s === p.fromLabel)
  );

  return (
    <div style={{ position: "fixed", top: 80, right: 24, background: "#fff", border: "1px solid #d1d5db", padding: 16, width: 360 }}>
      <h3>Link from {p.fromLabel} #{p.fromId}</h3>
      <button onClick={p.onClose} style={{ float: "right" }}>×</button>

      <p>Type: {allowedTypes.map(([t]) => (
        <button key={t} onClick={() => setType(t)} style={{ marginRight: 4, fontWeight: t === type ? "bold" : "normal" }}>{t}</button>
      ))}</p>

      {type && schema.rels[type] && (
        <TargetPicker
          allowedLabel={schema.rels[type].endpoints.find(([s]) => s === p.fromLabel)?.[1] ?? ""}
          onChoose={setTarget}
        />
      )}

      {type && target && (
        <EntityForm
          fields={schema.rels[type].props}
          submitLabel="Stage"
          onSubmit={(values) => { p.onStage(type, target.id, values); p.onClose(); }}
        />
      )}
    </div>
  );
}

function TargetPicker({ allowedLabel, onChoose }: { allowedLabel: string; onChoose: (e: { id: number; label: string }) => void }) {
  const [q, setQ] = useState("");
  const { data } = useEntities(allowedLabel, q);
  return (
    <div>
      <input value={q} onChange={(e) => setQ(e.target.value)} placeholder={`Search ${allowedLabel}…`} />
      <ul style={{ maxHeight: 200, overflow: "auto" }}>
        {data?.slice(0, 10).map((e) => (
          <li key={e.id} onClick={() => onChoose({ id: e.id, label: allowedLabel })} style={{ cursor: "pointer" }}>
            {String(e.props.name ?? e.id)}
          </li>
        ))}
      </ul>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add webui/src/components/RelationshipPicker.tsx
git commit -m "feat(webui): RelationshipPicker (type → target → props)"
```

---

## Task 22: webui — Routing shell (App.tsx)

**Files:**
- Modify: `webui/src/App.tsx`
- Create: `webui/src/AppProviders.tsx`

- [ ] **Step 1: `AppProviders.tsx`**

```tsx
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter } from "react-router-dom";
import type { ReactNode } from "react";

const qc = new QueryClient();

export default function AppProviders({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider client={qc}>
      <BrowserRouter>{children}</BrowserRouter>
    </QueryClientProvider>
  );
}
```

- [ ] **Step 2: Replace `App.tsx`**

```tsx
import { Routes, Route, Link, Navigate } from "react-router-dom";
import AppProviders from "./AppProviders";
import BrowsePage from "./routes/BrowsePage";
import EntityPage from "./routes/EntityPage";
import GraphPage from "./routes/GraphPage";
import SearchPage from "./routes/SearchPage";
import PendingPanel from "./components/PendingPanel";

export default function App() {
  return (
    <AppProviders>
      <div style={{ display: "grid", gridTemplateRows: "48px 1fr", height: "100vh" }}>
        <nav style={{ background: "#1f2937", color: "#f9fafb", display: "flex", alignItems: "center", gap: 16, padding: "0 16px" }}>
          <strong>kg editor</strong>
          <Link to="/browse" style={{ color: "#f9fafb" }}>Browse</Link>
          <Link to="/graph" style={{ color: "#f9fafb" }}>Graph</Link>
          <Link to="/search" style={{ color: "#f9fafb" }}>Search</Link>
        </nav>
        <main style={{ overflow: "auto" }}>
          <Routes>
            <Route path="/" element={<Navigate to="/browse" />} />
            <Route path="/browse" element={<BrowsePage />} />
            <Route path="/entity/:id" element={<EntityPage />} />
            <Route path="/graph" element={<GraphPage />} />
            <Route path="/search" element={<SearchPage />} />
          </Routes>
        </main>
        <PendingPanel />
      </div>
    </AppProviders>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add webui/src
git commit -m "feat(webui): router shell + AppProviders"
```

---

## Task 23: webui — BrowsePage

**Files:**
- Create: `webui/src/routes/BrowsePage.tsx`

- [ ] **Step 1: Implement**

```tsx
import { useSearchParams } from "react-router-dom";
import { useState } from "react";
import EntityListSidebar from "../components/EntityListSidebar";
import EntityList from "../components/EntityList";
import EntityForm from "../components/EntityForm";
import { useSchema } from "../hooks/useSchema";
import { useStore } from "../state/store";

export default function BrowsePage() {
  const [params, setParams] = useSearchParams();
  const label = params.get("label") ?? undefined;
  const showAdd = params.get("add") === "1";
  const { data: schema } = useSchema();
  const append = useStore((s) => s.append);
  const [tmpIdCounter, setTmpIdCounter] = useState(0);

  const closeAdd = () => {
    const p = new URLSearchParams(params);
    p.delete("add");
    setParams(p);
  };

  return (
    <div style={{ display: "grid", gridTemplateColumns: "240px 1fr", height: "100%" }}>
      <EntityListSidebar />
      <section style={{ padding: 16 }}>
        <h2>{label ? `${label} entities` : "All entities"}</h2>
        <EntityList label={label} />
        {showAdd && schema && label && schema.nodes[label] && (
          <div style={{ position: "fixed", right: 0, top: 48, width: 400, height: "100vh", background: "#fff", borderLeft: "1px solid #d1d5db", padding: 16, overflow: "auto" }}>
            <h3>New {label}</h3>
            <button onClick={closeAdd} style={{ float: "right" }}>×</button>
            <EntityForm
              fields={schema.nodes[label].props}
              submitLabel="Stage"
              onSubmit={(values) => {
                append({ kind: "create_node", tmpId: `t${tmpIdCounter}`, label, props: values });
                setTmpIdCounter(tmpIdCounter + 1);
                closeAdd();
              }}
            />
          </div>
        )}
      </section>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add webui/src/routes/BrowsePage.tsx
git commit -m "feat(webui): BrowsePage with sidebar + list + add drawer"
```

---

## Task 24: webui — EntityPage

**Files:**
- Create: `webui/src/routes/EntityPage.tsx`
- Create: `webui/src/hooks/useEntity.ts`

- [ ] **Step 1: `useEntity` hook**

```ts
import { useQuery } from "@tanstack/react-query";
import { getEntity } from "../kg/client";

export function useEntity(id: number) {
  return useQuery({ queryKey: ["entity", id], queryFn: () => getEntity(id) });
}
```

- [ ] **Step 2: `EntityPage.tsx`**

```tsx
import { useParams, Link } from "react-router-dom";
import { useState } from "react";
import { useEntity } from "../hooks/useEntity";
import { useSchema } from "../hooks/useSchema";
import { useStore } from "../state/store";
import EntityForm from "../components/EntityForm";
import RelationshipPicker from "../components/RelationshipPicker";

export default function EntityPage() {
  const { id } = useParams();
  const numId = Number(id);
  const { data: e } = useEntity(numId);
  const { data: schema } = useSchema();
  const append = useStore((s) => s.append);
  const [editing, setEditing] = useState(false);
  const [linking, setLinking] = useState(false);

  if (!e || !schema) return <p>Loading…</p>;
  const label = e.labels[0];
  const def = label ? schema.nodes[label] : undefined;

  return (
    <div style={{ padding: 24, maxWidth: 800, margin: "0 auto" }}>
      <h2>{label} #{e.id}</h2>

      <section>
        <h3>Properties</h3>
        {!editing ? (
          <>
            <dl>
              {Object.entries(e.props).map(([k, v]) => (
                <div key={k}><dt><strong>{k}</strong></dt><dd>{String(v)}</dd></div>
              ))}
            </dl>
            <button onClick={() => setEditing(true)}>Edit</button>{" "}
            <button onClick={() => append({ kind: "delete_node", id: numId, cascade: true })}>Delete entity</button>
          </>
        ) : def ? (
          <EntityForm
            fields={def.props}
            initial={e.props}
            submitLabel="Stage update"
            onSubmit={(values) => {
              const set: Record<string, unknown> = {};
              const unset: string[] = [];
              for (const [k, v] of Object.entries(values)) {
                if (v == null || v === "") unset.push(k);
                else set[k] = v;
              }
              append({ kind: "update_node", id: numId, set, unset });
              setEditing(false);
            }}
          />
        ) : <p>Unknown schema for label {label}</p>}
      </section>

      <section>
        <h3>Relationships</h3>
        <ul>
          {e.out_rels.map((r) => (
            <li key={r.id}>
              {r.type} → <Link to={`/entity/${r.target_id}`}>{r.target_labels[0]} #{r.target_id}</Link>
              {" "}<button onClick={() => append({ kind: "delete_link", id: r.id })}>×</button>
            </li>
          ))}
          {e.in_rels.map((r) => (
            <li key={r.id}>
              ← {r.type} <Link to={`/entity/${r.source_id}`}>{r.source_labels[0]} #{r.source_id}</Link>
              {" "}<button onClick={() => append({ kind: "delete_link", id: r.id })}>×</button>
            </li>
          ))}
        </ul>
        <button onClick={() => setLinking(true)}>+ Link</button>
      </section>

      {linking && label && (
        <RelationshipPicker
          fromId={numId}
          fromLabel={label}
          onClose={() => setLinking(false)}
          onStage={(type, targetId, props) => append({
            kind: "create_link",
            tmpId: `tl${Date.now()}`,
            type,
            start: { kind: "id", ref: numId },
            end:   { kind: "id", ref: targetId },
            props,
          })}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add webui/src
git commit -m "feat(webui): EntityPage with view/edit/delete + link picker"
```

---

## Task 25: webui — GraphPage + SearchPage

**Files:**
- Create: `webui/src/routes/GraphPage.tsx`
- Create: `webui/src/routes/SearchPage.tsx`
- Create: `webui/src/components/GraphCanvas.tsx`

- [ ] **Step 1: `GraphCanvas` (rebuild of phase-1 Canvas, schema-aware)**

```tsx
import { useEffect, useRef } from "react";
import cytoscape, { type Core, type ElementDefinition } from "cytoscape";
// @ts-ignore
import coseBilkent from "cytoscape-cose-bilkent";
cytoscape.use(coseBilkent);
import { useNavigate } from "react-router-dom";

interface GraphCanvasProps {
  nodes: Array<{ id: number; label: string; name: string }>;
  edges: Array<{ id: number; type: string; source: number; target: number }>;
}

export default function GraphCanvas({ nodes, edges }: GraphCanvasProps) {
  const ref = useRef<HTMLDivElement>(null);
  const cyRef = useRef<Core | null>(null);
  const navigate = useNavigate();

  useEffect(() => {
    if (!ref.current) return;
    const cy = cytoscape({
      container: ref.current,
      style: [
        { selector: "node", style: { label: "data(name)", "background-color": "data(color)", "font-size": 10, color: "#fff", "text-valign": "center", "text-halign": "center", width: 40, height: 40 } },
        { selector: "edge", style: { label: "data(type)", "curve-style": "bezier", "target-arrow-shape": "triangle", "line-color": "#9ca3af", "target-arrow-color": "#9ca3af", "font-size": 9 } },
      ] as unknown as cytoscape.StylesheetStyle[],
      layout: { name: "cose-bilkent", animate: false },
    });
    cy.on("tap", "node", (e) => navigate(`/entity/${e.target.data("entityId")}`));
    cyRef.current = cy;
    return () => { cy.destroy(); cyRef.current = null; };
  }, [navigate]);

  useEffect(() => {
    const cy = cyRef.current;
    if (!cy) return;
    const elems: ElementDefinition[] = [];
    for (const n of nodes) {
      elems.push({ group: "nodes", data: { id: `n${n.id}`, name: n.name, entityId: n.id, color: labelColor(n.label) } });
    }
    for (const e of edges) {
      elems.push({ group: "edges", data: { id: `e${e.id}`, source: `n${e.source}`, target: `n${e.target}`, type: e.type } });
    }
    cy.json({ elements: elems });
    cy.layout({ name: "cose-bilkent", animate: false }).run();
  }, [nodes, edges]);

  return <div ref={ref} style={{ width: "100%", height: "100%" }} />;
}

function labelColor(label: string): string {
  let h = 0;
  for (let i = 0; i < label.length; i++) h = (h * 31 + label.charCodeAt(i)) % 360;
  return `hsl(${h}, 50%, 45%)`;
}
```

- [ ] **Step 2: `GraphPage`**

```tsx
import { useState } from "react";
import { useSchema } from "../hooks/useSchema";
import { useEntities } from "../hooks/useEntities";
import { useQuery } from "@tanstack/react-query";
import GraphCanvas from "../components/GraphCanvas";

export default function GraphPage() {
  const { data: schema } = useSchema();
  const [labels, setLabels] = useState<Set<string>>(new Set());

  // Single page-wide fetch; per-label filtering is a future refinement.
  const { data: allNodes } = useEntities(undefined, undefined, 200);
  const { data: allEdges } = useQuery({
    queryKey: ["edges", Array.from(labels).sort()],
    queryFn: async () => {
      const r = await fetch(`${(import.meta.env.VITE_KG_SERVER_BASE as string) ?? "http://localhost:9000"}/query`, {
        method: "POST", headers: { "content-type": "application/json" },
        body: JSON.stringify({ cypher: "MATCH (s)-[r]->(e) RETURN id(r) AS id, type(r) AS type, id(s) AS source, id(e) AS target LIMIT 500", params: {} }),
      });
      const v = await r.json();
      // Unwrap PropValue envelope:
      const u = (x: unknown): unknown => {
        if (x && typeof x === "object" && "kind" in (x as object) && "value" in (x as object)) return (x as { value: unknown }).value;
        return x;
      };
      return (v.rows ?? []).map((row: Record<string, unknown>) => ({
        id: u(row.id) as number,
        type: u(row.type) as string,
        source: u(row.source) as number,
        target: u(row.target) as number,
      }));
    },
  });

  if (!schema) return <p>Loading…</p>;

  const filtered = (allNodes ?? []).filter((n) => labels.size === 0 || labels.has(n.labels[0])).map((n) => ({
    id: n.id, label: n.labels[0] ?? "?", name: String(n.props.name ?? n.id),
  }));
  const filteredEdges = (allEdges ?? []).filter((e) => {
    if (labels.size === 0) return true;
    const s = (allNodes ?? []).find((n) => n.id === e.source);
    const t = (allNodes ?? []).find((n) => n.id === e.target);
    return s && t && labels.has(s.labels[0]) && labels.has(t.labels[0]);
  });

  return (
    <div style={{ display: "grid", gridTemplateColumns: "200px 1fr", height: "100%" }}>
      <aside style={{ borderRight: "1px solid #d1d5db", padding: 12 }}>
        <h3>Labels</h3>
        {Object.keys(schema.nodes).map((l) => (
          <label key={l} style={{ display: "block" }}>
            <input type="checkbox" checked={labels.has(l)} onChange={(e) => {
              const next = new Set(labels);
              if (e.target.checked) next.add(l); else next.delete(l);
              setLabels(next);
            }} /> {l}
          </label>
        ))}
        <p style={{ fontSize: 12, color: "#6b7280" }}>Tick boxes to filter. Unticked = all.</p>
      </aside>
      <GraphCanvas nodes={filtered} edges={filteredEdges as never} />
    </div>
  );
}
```

- [ ] **Step 3: `SearchPage`**

```tsx
import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { search } from "../kg/client";

export default function SearchPage() {
  const [q, setQ] = useState("");
  const { data } = useQuery({
    queryKey: ["search", q],
    queryFn: () => search(q),
    enabled: q.length >= 1,
  });
  return (
    <div style={{ padding: 16, maxWidth: 800, margin: "0 auto" }}>
      <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search across all entities…" style={{ width: "100%", padding: 8 }} />
      <ul>
        {(data ?? []).map((e) => (
          <li key={e.id}>
            <Link to={`/entity/${e.id}`}>{e.labels[0]} — {String(e.props.name ?? e.id)}</Link>
          </li>
        ))}
      </ul>
    </div>
  );
}
```

- [ ] **Step 4: Commit**

```bash
cd webui && npm run build
git add webui/src
git commit -m "feat(webui): GraphPage (filterable cytoscape) + SearchPage"
```

---

## Task 26: webui — shadcn polish (vendored)

**Files:**
- Add: `webui/src/components/ui/` (vendored shadcn components)

Phase 2 keeps polish minimal but standardized. Run shadcn CLI to vendor:

```bash
cd webui
npx shadcn@latest init --yes --base-color slate
npx shadcn@latest add button input combobox dialog
```

Migrate buttons / inputs in: `EntityForm`, `PendingPanel`, `Toolbar` (if any
remain) — to use the vendored components. Skip migration for any component
that hasn't been touched yet in phase 2 if time-boxing.

Commit:
```bash
git add webui
git commit -m "feat(webui): vendor shadcn/ui base components"
```

---

## Task 27: webui — Vitest + Playwright

**Files:**
- Modify: `webui/src/tests/store.test.ts`
- Create: `webui/tests/e2e/sme-workflow.spec.ts`
- Modify: `webui/package.json`
- Create: `webui/playwright.config.ts`

- [ ] **Step 1: Update store test**

Rewrite for the new minimal store (3 actions).

```ts
import { describe, expect, it, beforeEach } from "vitest";
import { useStore } from "../state/store";

describe("pending store", () => {
  beforeEach(() => useStore.setState({ pending: [] }));

  it("append adds an op", () => {
    useStore.getState().append({ kind: "create_node", tmpId: "t1", label: "Person", props: { name: "Alice" } });
    expect(useStore.getState().pending).toHaveLength(1);
  });

  it("remove by index drops only that op", () => {
    useStore.getState().append({ kind: "create_node", tmpId: "t1", label: "Person", props: {} });
    useStore.getState().append({ kind: "create_node", tmpId: "t2", label: "Person", props: {} });
    useStore.getState().remove(0);
    expect(useStore.getState().pending).toHaveLength(1);
    expect((useStore.getState().pending[0] as { tmpId: string }).tmpId).toBe("t2");
  });

  it("clear empties", () => {
    useStore.getState().append({ kind: "create_node", tmpId: "t1", label: "Person", props: {} });
    useStore.getState().clear();
    expect(useStore.getState().pending).toEqual([]);
  });
});
```

- [ ] **Step 2: Playwright config + test**

```bash
cd webui && npm install --save-dev @playwright/test
npx playwright install chromium
```

`webui/playwright.config.ts`:
```ts
import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./tests/e2e",
  use: { baseURL: "http://localhost:5173" },
  webServer: { command: "npm run dev", port: 5173, reuseExistingServer: true },
});
```

`webui/tests/e2e/sme-workflow.spec.ts`:
```ts
import { test, expect } from "@playwright/test";

test("create person via form, commit, see in list", async ({ page }) => {
  await page.goto("/browse?label=Person&add=1");
  await page.getByLabel(/name/i).fill("Phyllis");
  await page.getByLabel(/role/i).selectOption("engineer");
  await page.getByRole("button", { name: /stage/i }).click();
  await page.getByRole("button", { name: /commit all/i }).click();
  // expect refresh; Phyllis appears
  await expect(page.getByText("Phyllis")).toBeVisible({ timeout: 10000 });
});
```

This test requires kg-server + Neo4j running. CI will provide both via job
services; locally the dev environment must be up.

Add to `webui/package.json` scripts:
```json
"e2e": "playwright test"
```

- [ ] **Step 3: Commit**

```bash
git add webui
git commit -m "test(webui): rewrite store tests for new shape; add Playwright SME smoke"
```

---

## Task 28: kg-server — extended integration tests

**Files:**
- Modify: `crates/kg-server/tests/server.rs`

Add tests covering /entities (list+get), /links, /search alongside the create/update/delete tests added piecewise in T7-T10. Make the file the canonical smoke battery for the new API surface.

- [ ] **Step 1: Add 4 more tests**

(One per: list, get_one, link create+delete, search.)

- [ ] **Step 2: Run + commit**

```bash
export DOCKER_HOST=unix://${HOME}/.colima/default/docker.sock
cargo test -p kg-server -- --test-threads=1
git add crates/kg-server/tests
git commit -m "test(kg-server): list/get/link/search integration tests"
```

---

## Task 29: docs — manual UI smoke + MCP smoke

**Files:**
- Modify: `docs/manual-ui-smoke.md`
- Create: `docs/manual-mcp-smoke.md`

- [ ] **Step 1: Replace `manual-ui-smoke.md` with the SME flow**

Updated steps: open /browse → click + Add Person → fill form (name+role+age) → Stage → Commit → see in list. Edit. Delete. Link via /entity/:id RelationshipPicker. Repeat the table at bottom.

- [ ] **Step 2: New `manual-mcp-smoke.md`**

```markdown
# Manual MCP smoke (one-time per release)

## Prereqs
- Docker (`docker run -d --name kg-neo4j …`)
- kg-server running on :9000 with KG_SCHEMA set
- Claude Desktop or Claude Code with MCP support
- `cargo install --path crates/kg-mcp` (or use `cargo run -p kg-mcp` directly)

## Steps

1. Add to your Claude config (`~/Library/Application Support/Claude/claude_desktop_config.json` or equivalent):
   ```json
   {
     "mcpServers": {
       "kg-editor": {
         "command": "cargo",
         "args": ["run", "-p", "kg-mcp", "--manifest-path", "/abs/path/to/kg_editor/Cargo.toml"],
         "env": { "KG_SERVER_URL": "http://localhost:9000" }
       }
     }
   }
   ```
2. Restart Claude. Confirm the `kg-editor` server appears in the MCP list.
3. In a fresh conversation, ask: "What labels are in the kg schema?" — Claude should invoke `get_schema` and answer with the Person/Company/Skill list.
4. Ask: "Create a Person named Quincy, age 41, role manager." — Claude invokes `create_entity` and confirms the new id.
5. Ask: "Link Quincy to Acme via WORKS_AT with role engineer." — Claude calls `find_entities` to resolve both ids, then `link_entities`.
6. Ask: "Find everyone whose name starts with Q." — `find_entities` returns Quincy.

Record:

| Date | Engineer | MCP client | Pass/Fail | Notes |
|------|----------|------------|-----------|-------|
| YYYY-MM-DD | name | Claude Code 1.x | | |
```

- [ ] **Step 3: Commit**

```bash
git add docs
git commit -m "docs: phase-2 SME UI smoke + MCP smoke checklists"
```

---

## Task 30: CI additions

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Append jobs**

```yaml
  test-kg-schema:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.95.0
      - uses: Swatinem/rust-cache@v2
      - run: cargo test -p kg-schema

  test-kg-mcp:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.95.0
      - uses: Swatinem/rust-cache@v2
      - run: cargo test -p kg-mcp

  webui-e2e:
    runs-on: ubuntu-latest
    services:
      neo4j:
        image: neo4j:5-community
        ports: ["7687:7687"]
        env: { NEO4J_AUTH: neo4j/testtest }
        options: >-
          --health-cmd="cypher-shell -u neo4j -p testtest 'RETURN 1' || exit 1"
          --health-interval=10s --health-timeout=5s --health-retries=20
    env:
      NEO4J_URI: bolt://127.0.0.1:7687
      NEO4J_PASSWORD: testtest
      KG_SCHEMA: ${{ github.workspace }}/kg-schema.yaml
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - uses: dtolnay/rust-toolchain@1.95.0
        with: { targets: wasm32-unknown-unknown }
      - uses: Swatinem/rust-cache@v2
      - run: cd webui && npm ci
      - run: cd webui && npx playwright install --with-deps chromium
      - run: cargo run -p kg-server &
      - run: cd webui && npm run e2e
```

Update the existing `test-kg-server` job to set `KG_SCHEMA` env to a path of a schema file (commit a `kg-schema.yaml` in repo root in T4).

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: phase-2 jobs (kg-schema, kg-mcp, webui-e2e)"
```

---

## Task 31: README phase-2 section

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Append phase-2 to the Build matrix**

```markdown
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
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: README phase-2 build matrix"
```

---

## Self-Review

**Spec coverage walk-through:**

| Spec § | Requirement | Plan task(s) |
|---|---|---|
| §4 | kg-schema.yaml top-level file | 4 |
| §5 | kg-schema crate (FieldSpec, FieldType, parse YAML, validate, to_core_registry) | 2–4 |
| §6 | /schema + /entities (list/get/create/update/delete) + /links + /search | 5–10 |
| §6 | Per-op REST dispatch instead of WASM staging on SME path | 17, 20 |
| §7 | webui pages: BrowsePage / EntityPage / GraphPage / SearchPage | 22–25 |
| §7 | EntityListSidebar + EntityList | 19 |
| §7 | EntityForm schema-driven | 18 |
| §7 | RelationshipPicker | 21 |
| §7 | PendingPanel | 20 |
| §7 | GraphCanvas rebuild | 25 |
| §7 | shadcn polish | 26 |
| §8 | kg-mcp crate (scaffold + 7 tools + subprocess test) | 11–13 |
| §10 T1 | kg-schema unit tests | 4 |
| §10 T2 | kg-server extended integration tests | 7, 8, 9, 10, 28 |
| §10 T3 | kg-mcp subprocess test | 13 |
| §10 T4 | webui Vitest + Playwright | 27 |
| §10 T5 | manual smokes | 29 |
| §11 | make targets — KG_SCHEMA env, kg-mcp run instructions | 31 |
| §12 | CI additions | 30 |
| §13 acceptance | SME create / link / edit / delete with zero Cypher | 18, 20, 23, 24 |
| §13 acceptance | AI workflow via MCP | 11–13 |

**Placeholder scan:** No "TBD"/"TODO". Two notes ("adjust rmcp version", "vendor shadcn via CLI") are concrete instructions, not placeholders.

**Type consistency:** TS `FieldType` mirrors Rust `FieldType` via serde tag=type. `SchemaFile`, `NodeDef`, `RelDef`, `FieldSpec`, `Cardinality` consistent across crates and UI.

**Scope check:** One coherent project (SME UI + AI MCP on top of phase-1). Manageable.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-20-phase-2-sme-ai.md`. Two execution options:

1. **Subagent-Driven (recommended)** — fresh subagent per task, two-stage review between tasks. Uses `superpowers:subagent-driven-development`.
2. **Inline Execution** — execute tasks in this session using `superpowers:executing-plans`, batched with checkpoints.

Which approach?
