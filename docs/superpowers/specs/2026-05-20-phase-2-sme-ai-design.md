# Phase 2 — SME-friendly UI + AI MCP Integration

**Date:** 2026-05-20
**Status:** Design — pending user approval
**Owner:** plutonium-guy
**Depends on:** phase-1/ui (merged or branch)

## 1. Purpose

Replace the phase-1 dev-shaped UI (window.prompt dialogs, shift-drag gestures,
raw labels) with a workflow tailored to **subject-matter experts (SMEs)** who
should never need to write Cypher. Add an **MCP server** so AI assistants can
drive the same graph using structured intent operations, also without writing
Cypher. Both surfaces sit on top of a single shared `kg-schema` registry
loaded from a YAML file at server startup.

This phase intentionally excludes: multi-user / auth, real-time collaboration,
schema editing UI (schema is file-driven), undo across server restarts,
i18n / accessibility audit, and mobile. Those are phase 3+.

## 2. Goals & Non-goals

### Goals

- An SME can browse, create, edit, link, and delete graph entities through
  forms and clicks. No Cypher exposed.
- Forms are auto-generated from a YAML schema declaring labels, props (with
  type + required flag + optional enum / ref to other label), and allowed
  relationship endpoints.
- Pending mutations are visible as a list ("Create Person 'Alice'",
  "Link Alice→Bob (KNOWS)") that can be discarded individually before commit.
- AI agents can connect via **MCP stdio** and call tools `create_entity`,
  `update_entity`, `delete_entity`, `link_entities`, `unlink_entities`,
  `find_entities`, `get_schema` — none of which require the agent to write
  Cypher.
- Same kg-server backs both surfaces; schema is loaded once at boot.
- Visual graph remains useful: stable positions persisted in localStorage,
  zoom-to-fit, search highlights, label-colored nodes.

### Non-goals

- Auth / multi-user / authorization.
- Schema editing through the UI (schema lives in `kg-schema.yaml`).
- Undo / redo across server restarts (in-session undo OK as nice-to-have).
- Mobile / touch.
- WCAG audit.
- Real-time collaborative editing.

## 3. High-Level Architecture

Three new components, one new shared crate:

```
kg_editor/
├── crates/
│   ├── kg-core/                phase-0
│   ├── kg-neo4j/               phase-0
│   ├── kg-core-wasm/           phase-1
│   ├── kg-schema/              NEW: YAML schema loader + validator on top of kg-core::SchemaRegistry
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs          parse YAML -> SchemaRegistry, plus richer FieldSpec
│   │       └── field.rs        FieldSpec (type+required+enum+ref) shared by SchemaRegistry-like extensions
│   ├── kg-server/              EXTENDED: schema loading + new endpoints + entity helpers
│   │   ├── src/routes/
│   │   │   ├── schema.rs       GET /schema
│   │   │   ├── entities.rs     POST /entities, PUT /entities/{id}, DELETE /entities/{id}, POST /links, DELETE /links/{id}
│   │   │   └── search.rs       GET /search?label=&q=&limit=
│   │   └── ...
│   └── kg-mcp/                 NEW: MCP stdio server
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           └── tools.rs        create/update/delete/link/unlink/find/get_schema MCP tools
├── webui/                      REWRITTEN: forms-first UX
│   └── src/
│       ├── pages/              new
│       │   ├── BrowsePage.tsx
│       │   ├── EntityPage.tsx
│       │   └── GraphPage.tsx
│       ├── components/         most existing replaced
│       │   ├── EntityForm.tsx
│       │   ├── EntityList.tsx
│       │   ├── PendingPanel.tsx
│       │   ├── RelationshipPicker.tsx
│       │   └── GraphCanvas.tsx
│       └── kg/                 schema-aware glue
├── kg-schema.yaml              NEW: top-level schema definition file
└── docs/
    └── superpowers/specs/      this doc + earlier
```

## 4. `kg-schema.yaml`

Example shape — `kg-schema` crate reads this at server startup. All fields
are optional unless `required: true`. Field types:
`string | int | float | bool | date | datetime | enum | ref`. `ref` points to
another label by name and is rendered in the UI as an entity picker.

```yaml
nodes:
  Person:
    description: An individual person.
    props:
      - name:        { type: string, required: true, unique: true }
      - age:         { type: int }
      - role:        { type: enum, values: [engineer, designer, manager, cto, other] }
      - email:       { type: string }
      - joined:      { type: date }
    indexes:
      - [name]
  Company:
    description: A company that employs people.
    props:
      - name:        { type: string, required: true, unique: true }
      - founded:     { type: int }
      - website:     { type: string }
  Skill:
    description: A discrete skill or competency.
    props:
      - name:        { type: string, required: true, unique: true }
      - category:    { type: enum, values: [language, framework, soft, domain] }

rels:
  KNOWS:
    endpoints: [[Person, Person]]
    cardinality: many_to_many
    props:
      - since:       { type: int }
      - strength:    { type: float }
  WORKS_AT:
    endpoints: [[Person, Company]]
    cardinality: many_to_many
    props:
      - role:        { type: string }
      - start_date:  { type: date }
      - end_date:    { type: date }
  HAS_SKILL:
    endpoints: [[Person, Skill]]
    cardinality: many_to_many
    props:
      - level:       { type: enum, values: [novice, intermediate, expert] }
```

Server boots with `KG_SCHEMA=/path/to/kg-schema.yaml`. Missing file → server
refuses to start with a clear error.

## 5. `kg-schema` crate

```rust
// kg-schema/src/lib.rs

pub struct SchemaFile {
    pub nodes: HashMap<String, NodeDef>,
    pub rels:  HashMap<String, RelDef>,
}

pub struct NodeDef {
    pub description: Option<String>,
    pub props: Vec<FieldSpec>,
    pub indexes: Vec<Vec<String>>,
}

pub struct RelDef {
    pub endpoints: Vec<(String, String)>,
    pub cardinality: Cardinality,
    pub props: Vec<FieldSpec>,
}

pub struct FieldSpec {
    pub name: String,
    pub ty: FieldType,
    pub required: bool,
    pub unique: bool,
    pub default: Option<JsonValue>,
}

pub enum FieldType {
    String, Int, Float, Bool, Date, DateTime,
    Enum { values: Vec<String> },
    Ref  { label: String },
}

impl SchemaFile {
    pub fn from_yaml(path: &Path) -> Result<Self, SchemaError>;
    /// Project down to the existing kg-core::SchemaRegistry for validation
    /// reuse. Enum + Ref get flattened to String + Int respectively in the
    /// core registry; the richer info stays in SchemaFile for UI form gen.
    pub fn to_core_registry(&self) -> kg_core::schema::SchemaRegistry;
}
```

The `kg-server` keeps both:
- `Arc<SchemaFile>` for serving `/schema` to the UI and MCP.
- `Arc<SchemaRegistry>` for kg-core validation on commit.

## 6. New `kg-server` endpoints

All endpoints below sit alongside the phase-1 `/health`, `/query`, `/commit`.
They are **convenience wrappers** that build the same EmitOutput shape that
`/commit` accepts. The UI uses these directly instead of staging through
WASM, removing the WASM dependency from the SME path. (WASM stays for the
power-user "raw Cypher box" surface, hidden behind a settings toggle in
phase 2.)

```
GET  /schema                   -> SchemaFile JSON (nodes, rels, fields)
GET  /entities?label=Person&q=ali&limit=50
                               -> [{id, labels, props}, ...]
GET  /entities/:id             -> {id, labels, props, in_rels:[], out_rels:[]}
POST /entities                 -> {label, props}
                               -> 201 {id, ...}
PUT  /entities/:id             -> {patch:{set:{...}, unset:[...]}}
                               -> 200
DELETE /entities/:id?cascade=true|false
                               -> 204
POST /links                    -> {type, start_id, end_id, props}
                               -> 201 {id}
DELETE /links/:id              -> 204
GET  /search?q=ali&limit=20    -> [{kind:"node"|"rel", id, label/type, summary, props}]
```

All requests body-level validate against `SchemaFile` before issuing
Cypher; violations return 422 with a structured error list the UI can render
next to specific fields.

The `pending` concept (multiple staged ops committed together) becomes a
client-side concern. UI keeps an array of pending ops, sends them one at a
time on Commit. Server stays stateless.

## 7. webui rewrite

### 7.1 Tech stack changes

Keep: Vite + React + TS + Zustand + Cytoscape. Add:

- **react-router-dom** for `/browse`, `/entity/:id`, `/graph`, `/search` routes
- **react-hook-form** + **zod** for schema-driven forms
- **@tanstack/react-query** for server cache + invalidation
- **shadcn/ui** components for visual polish (button, dialog, input,
  combobox, table, sidebar) — vendored via the CLI generator, not as a
  runtime dep
- Drop the old shift-drag / cxttap interactions

### 7.2 Pages

**`/browse`** (default landing)
```
+----------------------------+--------------------------------+
| Sidebar:                   | Entity list                    |
|   ▾ Person       (52)      |  Search [_____________ ]       |
|     • Alice                |                                 |
|     • Bob                  |  Name      Role       Joined    |
|     • ...                  |  ───────── ────────── ────────  |
|   ▾ Company      (8)       |  Alice     engineer   2020-01   |
|   ▾ Skill        (14)      |  Bob       designer   2021-03   |
|                            |  ...                            |
|   [ + Add Person ]         |                                 |
|   [ + Add Company ]        |  Pagination · 50 of 52          |
+----------------------------+--------------------------------+
```

Click a row → `/entity/:id`. Click "+ Add Person" → opens a side drawer with
a Person form.

**`/entity/:id`**
```
+----------------------------+-----------------------------------+
| Sidebar:                   | Entity: Alice (Person)            |
|   ▾ Person                 |                                   |
|   ...                      | Properties               [Edit]   |
|                            |   name      Alice                 |
|                            |   age       30                    |
|                            |   role      engineer              |
|                            |   email     alice@acme.com        |
|                            |                                   |
|                            | Relationships            [+ Link] |
|                            |   KNOWS → Bob (since 2020)        |
|                            |   KNOWS → Carol                   |
|                            |   WORKS_AT → Acme (role engineer) |
|                            |   ← KNOWS Dave (since 2022)       |
|                            |   HAS_SKILL → Rust                |
|                            |                                   |
|                            | [Delete entity]                   |
+----------------------------+-----------------------------------+
```

`[+ Link]` opens a relationship dialog:
1. **Type picker** — only types whose endpoints include this entity's label.
2. **Direction picker** — if both directions valid.
3. **Target picker** — typeahead over entities of the allowed label
   (calls `/entities?label=Person&q=...`).
4. **Props form** — auto-generated from rel schema.
5. **Stage** → adds to PendingPanel.

**`/graph`**
```
+----------------------------+-----------------------------------+
| Filter sidebar:            |                                   |
|   Labels                   |     Cytoscape canvas              |
|     ☑ Person               |     (forced layout, zoom-to-fit)  |
|     ☑ Company              |                                   |
|     ☐ Skill                |     • node = label-colored        |
|   Rel types                |     • selected highlights both    |
|     ☑ KNOWS                |       node + its incident rels    |
|     ☑ WORKS_AT             |                                   |
|     ☐ HAS_SKILL            |     mini-map (bottom-right)       |
|   Search highlight         |                                   |
|     [ ali__________ ]      |                                   |
|                            |                                   |
+----------------------------+-----------------------------------+
```

Clicking a node here jumps to `/entity/:id`.

**`/search`** — full-text search across all entities, results grouped by
label, each row links to `/entity/:id`.

### 7.3 Pending Panel

Sticky bottom bar visible on every page when `pendingCount > 0`:

```
+------------------------------------------------------------+
| 3 pending changes                                          |
|   • Create Person "Frank"                            [×]   |
|   • Update Alice age 30→31                           [×]   |
|   • Link Alice → Frank (KNOWS, since 2024)           [×]   |
|                              [Discard all]  [Commit all]   |
+------------------------------------------------------------+
```

Each `×` discards just that op. The "Commit all" button POSTs each op to the
relevant endpoint (`/entities`, `/entities/:id`, `/links`) in order, halting
on first error.

### 7.4 Form generation

Per FieldSpec.ty:

| FieldType | UI control |
|---|---|
| string | `<input type="text">` |
| int | `<input type="number" step=1>` |
| float | `<input type="number" step=0.01>` |
| bool | toggle |
| date | `<input type="date">` |
| datetime | `<input type="datetime-local">` |
| enum | shadcn Combobox over `values` |
| ref | typeahead over `/entities?label={target}` |

Required fields show an asterisk and validate via zod schema generated from
the FieldSpec.

### 7.5 Visual graph polish

- Cytoscape with `cose-bilkent` default layout, plus `dagre` and `concentric`
  options.
- Node colors keyed to label via a stable hash → HSL.
- Positions persisted per-user in localStorage keyed by entity id; layout
  only runs for nodes without saved positions.
- Mini-map via `cytoscape-navigator`.
- Search highlights pulse the matching nodes.
- Zoom-to-fit button on toolbar.

## 8. `kg-mcp` crate

A standalone binary that speaks MCP stdio (per the spec at
modelcontextprotocol.io). Tools:

```
create_entity(label, props) -> {id}
update_entity(id, set?, unset?) -> {ok}
delete_entity(id, cascade=false) -> {ok}
link_entities(type, start_id, end_id, props?) -> {id}
unlink_entities(rel_id) -> {ok}
find_entities(label?, query?, limit=20) -> [...]
get_schema() -> SchemaFile JSON
```

The crate uses `reqwest` to call kg-server (configured via `KG_SERVER_URL`
env var, default `http://localhost:9000`). No direct Neo4j access from MCP.

The binary launches with `cargo run -p kg-mcp`. To wire into Claude Desktop /
Code, a small `mcp.json` snippet is documented in
`docs/manual-mcp-smoke.md`.

## 9. Pending ops & ordering

The phase-1 UoW chained everything via WITH * inside a single Cypher
statement. Phase 2 dispatches each pending op as a discrete REST call. This
loses single-tx atomicity in exchange for per-op error messaging that's
useful to SMEs.

Order rules for "Commit all":
1. Schema/DDL ops (none in phase 2 from UI — schema is file-driven).
2. Node creates (server returns ids, used to resolve subsequent links).
3. Node updates.
4. Link creates.
5. Link deletes.
6. Node deletes.

If a step fails, the UI shows the partial-success state and lets the user
retry or discard the rest.

## 10. Testing strategy

### Tier 1 — `kg-schema`
Unit tests for YAML parsing: every FieldType, missing-required, malformed,
ref to undefined label, enum with empty values list.

### Tier 2 — `kg-server` extensions
Integration tests against testcontainers Neo4j:
- `/schema` returns the loaded YAML.
- `POST /entities` with valid + invalid bodies (missing required, wrong
  type, unknown label).
- `PUT /entities/:id` with set + unset patches.
- `POST /links` with allowed + disallowed endpoint pairs.
- `GET /entities?label=&q=&limit=` returns expected matches.

### Tier 3 — `kg-mcp`
Subprocess tests that spawn the MCP binary, send JSON-RPC over stdio, and
assert tool responses. Mock kg-server with a recording HTTP mock so the
MCP tests don't need Neo4j.

### Tier 4 — webui
- Vitest unit tests for the form generator (one test per FieldType).
- Playwright smoke: full SME workflow — open `/browse`, click "+ Add
  Person", fill form, stage, commit, verify entity appears.

### Tier 5 — Manual
`docs/manual-ui-smoke.md` extended for SME flows; `docs/manual-mcp-smoke.md`
new file with claude-desktop wiring + a smoke conversation.

## 11. Build / run

```
make neo4j-up
make wasm-build                     # phase-1 still — used by raw-cypher panel
KG_SCHEMA=$(pwd)/kg-schema.yaml NEO4J_PASSWORD=testtest cargo run -p kg-server
cd webui && npm run dev
# AI side, separate terminal:
KG_SERVER_URL=http://localhost:9000 cargo run -p kg-mcp
```

## 12. CI additions

- `kg-schema` unit tests.
- `kg-server` extended integration tests (replace existing job, same env).
- `kg-mcp` subprocess tests.
- `webui` extended Vitest + Playwright smoke.

## 13. Phase-2 Acceptance Criteria

1. SME workflow: open the SPA, click "+ Add Person" from the sidebar, fill
   the auto-generated form (with enum dropdown for role and date picker for
   joined), stage, commit. The new Person appears in the Person list.
2. Linking: open Alice, click "+ Link", pick `WORKS_AT`, search for "Acme",
   fill role + start_date, stage, commit. The link is visible on Alice's
   page and on Acme's page.
3. Editing: open Alice, click Edit on Properties, change `age` from 30 to
   31, save, commit. The change persists.
4. Deletion: open a no-longer-needed Person, click Delete, confirm,
   commit. The entity and its incident rels are gone.
5. Cypher knowledge required to do any of 1–4: **zero**.
6. AI workflow: Claude Code connects to `kg-mcp` over stdio, calls
   `get_schema`, then calls `create_entity` for a Person, then
   `link_entities` for a KNOWS, then `find_entities` to verify. The
   transcript is recorded in `docs/manual-mcp-smoke.md`.
7. All phase-0 and phase-1 tests still pass.
8. `kg-schema` ≥ 80% line coverage. New kg-server endpoints have
   integration tests against a real Neo4j.

## 14. Out of Scope / Phase 3+

- Auth / multi-user.
- Schema editing UI.
- Real-time collaborative editing.
- Mobile / touch.
- Accessibility audit.
- Data import (CSV / Parquet / JSON Lines).
- Saved queries / dashboards.
- Audit log / change history.
- Schema migrations.
- Embedded vector search.

## 15. Open Questions

- Whether to expose the phase-1 raw-Cypher panel as an "advanced" toggle in
  phase 2 (current plan: yes, but hidden by default).
- Whether `kg-server` should serve the built `webui/dist` directly in
  production (current plan: yes, via `KG_SERVE_UI` env var introduced but
  unused in phase 1).
- Whether to add a single-undo button in the PendingPanel before commit
  (current plan: yes — undo restores the most recent discarded op).
