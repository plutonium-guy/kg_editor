# Phase 1 — Browser UI for kg_editor

**Date:** 2026-05-20
**Status:** Design — pending user approval
**Owner:** plutonium-guy
**Depends on:** phase-0/kg-editor (merged or branch)

## 1. Purpose

Add a browser-based visual editor on top of the phase-0 Rust framework. Users
open the SPA in a browser, see a Neo4j graph rendered as a Cytoscape canvas,
edit nodes and relationships through direct-manipulation gestures and an
inspector panel, and run ad-hoc Cypher queries. Mutations are staged
client-side via the existing Unit-of-Work API, then committed through a thin
Rust HTTP proxy that fronts `kg-neo4j` native (Bolt).

This phase intentionally excludes auth, multi-user sync, real-time
collaboration, undo/redo history, schema editing UI, and visual layout
algorithms beyond Cytoscape's built-ins. Those are phase 2+.

## 2. Goals & Non-goals

### Goals

- A working browser SPA that renders a Neo4j graph and lets a single dev edit
  it interactively.
- Reuse phase-0 logic with zero duplication: client-side staging uses the same
  `UnitOfWork` and `CypherEmitter` as the native build, compiled to WASM.
- A small Rust HTTP server (`kg-server`) that translates JSON request bodies
  into `kg-neo4j::Client` calls. No business logic in the server.
- Direct-manipulation editing: drag nodes to reposition, drag from a node to
  another to create a rel, right-click empty canvas to add a node, click a
  node/rel to view + edit its props.
- A Cypher textarea where the user types a query, hits Run, and the result
  rows are visualized on the canvas (any returned nodes/rels merged into the
  current view; non-graph rows shown in a results table below the canvas).

### Non-goals

- Authentication / multi-user / authorization.
- Collaboration / real-time sync.
- Undo / redo history (only single-step revert before commit).
- Schema editor (registry is configured server-side, not in the UI).
- Custom layout algorithms beyond Cytoscape's built-ins (cose, dagre, grid).
- Mobile / touch input optimization.
- Offline mode.
- Importing CSVs or files into the graph through the UI.

## 3. High-Level Architecture

Three new components added to the phase-0 workspace:

```
kg_editor/
├── crates/
│   ├── kg-core/           # phase-0
│   ├── kg-neo4j/          # phase-0
│   ├── kg-core-wasm/      # NEW: wasm-bindgen wrapper
│   │   ├── Cargo.toml
│   │   └── src/lib.rs     # exposes PropValue, Node, Rel, UnitOfWork,
│   │                      # SchemaRegistry, CypherEmitter::emit
│   └── kg-server/         # NEW: axum HTTP proxy
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── routes.rs  # /commit, /query, /health
│           └── state.rs   # holds kg-neo4j::Client
├── webui/                 # NEW: Vite + React + TypeScript SPA
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   ├── public/
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── kg/             # thin TS wrapper around kg-core-wasm
│       │   ├── client.ts   # fetch wrappers around kg-server
│       │   └── uow.ts      # convenience builders calling into wasm
│       ├── components/
│       │   ├── Canvas.tsx       # Cytoscape canvas
│       │   ├── Inspector.tsx    # selected node/rel prop editor
│       │   ├── QueryBox.tsx     # Cypher textarea + Run button
│       │   ├── ResultsTable.tsx # non-graph query rows
│       │   └── Toolbar.tsx
│       ├── state/
│       │   └── store.ts    # Zustand store
│       └── styles/
└── docs/
    └── superpowers/specs/  # phase-0 + this doc
```

Data flow on a typical edit:

```
+---------------+        +-----------------+        +-------------+        +--------+
|  Cytoscape    |  user  |  React (Zustand)|  call  |  kg-core    |  emit  |  fetch |
|  canvas       | <----> |  view + state   | <----> |  -wasm UoW  | -----> |  POST  |
+---------------+        +-----------------+        +-------------+        +---+----+
                                                                              |
                                                                              v
                                                                    +---------+--------+
                                                                    |  kg-server       |
                                                                    |  axum (Rust)     |
                                                                    +---------+--------+
                                                                              |
                                                                              | kg-neo4j Bolt
                                                                              v
                                                                    +---------+--------+
                                                                    |   Neo4j 5        |
                                                                    +------------------+
```

## 4. `kg-core-wasm` crate

Thin wrapper around `kg-core` exposing wasm-bindgen bindings.

### 4.1 Bindings

`#[wasm_bindgen]` types and functions, surfaced to JS as:

```typescript
// auto-generated .d.ts after wasm-pack build
export class WasmPropValue { /* opaque */ }
export function prop_null(): WasmPropValue;
export function prop_bool(v: boolean): WasmPropValue;
export function prop_int(v: bigint): WasmPropValue;
export function prop_float(v: number): WasmPropValue;
export function prop_string(v: string): WasmPropValue;
export function prop_list(items: WasmPropValue[]): WasmPropValue;
export function prop_map(entries: Array<[string, WasmPropValue]>): WasmPropValue;

export class WasmUow {
  constructor();
  create_node(labels: string[], props: Array<[string, WasmPropValue]>): bigint;
  merge_node(labels: string[], key_props: Array<[string, WasmPropValue]>, set_props: Array<[string, WasmPropValue]>): bigint;
  update_node(server_id: bigint, sets: Array<[string, WasmPropValue]>, unsets: string[]): void;
  delete_node(server_id: bigint, detach: boolean): void;

  create_rel(start: NodeRefJs, end: NodeRefJs, ty: string, props: Array<[string, WasmPropValue]>): bigint;
  merge_rel(start: NodeRefJs, end: NodeRefJs, ty: string, key_props: Array<[string, WasmPropValue]>, set_props: Array<[string, WasmPropValue]>): bigint;
  update_rel(rel_id: bigint, sets: Array<[string, WasmPropValue]>, unsets: string[]): void;
  delete_rel(rel_id: bigint): void;

  /// Returns the EmitOutput { ddl, data } as a plain JS object via serde-wasm-bindgen.
  emit(): EmitOutputJs;
}

export type NodeRefJs =
  | { kind: "Server", id: bigint }
  | { kind: "Local", id: bigint };

export type EmitOutputJs = {
  ddl: Array<{ cypher: string; params: Record<string, unknown> }>;
  data: { cypher: string; params: Record<string, unknown> } | null;
};

export class WasmSchemaRegistry {
  constructor();
  add_node_schema(label: string, props: Array<PropSpecJs>, unique: string[][]): void;
  add_rel_schema(ty: string, endpoints: Array<[string, string]>, props: Array<PropSpecJs>): void;
  validate(uow: WasmUow): ValidationReport;
}
```

Notes:
- `PropValue` itself is opaque on the JS side; helpers (`prop_int`, etc) construct it. This avoids round-trip serialization on every prop set.
- `WasmUow.emit()` returns the EmitOutput as a plain JS object using `serde-wasm-bindgen`, which JS code then sends straight to the server.
- `Vec<u8>` (Bytes) is omitted in phase 1 (matches phase-0 conversion gap).

### 4.2 Build

```
wasm-pack build crates/kg-core-wasm --target web --out-dir ../../webui/pkg
```

The output is consumed by webui's Vite via `import init, * as kg from "../pkg/kg_core_wasm";`.

## 5. `kg-server` crate

A thin axum HTTP server. No business logic; every endpoint dispatches to
`kg-neo4j::Client` and returns the result as JSON.

### 5.1 Endpoints

```
GET  /health              -> 200 OK, body: {"status":"ok","neo4j":"reachable"|"down"}
POST /query               -> body: {"cypher":"...","params":{...}}
                             returns: {"rows":[{"col":<json>}, ...]}
POST /commit              -> body: EmitOutput JSON (same shape as kg-core-wasm emit)
                             returns: {"counters": {...}}
```

The body of `/commit` is exactly what `kg-core-wasm::WasmUow::emit()` produces.
The server iterates `output.ddl` (each run as an auto-commit) then runs
`output.data` (if present) inside a single transaction — mirroring
`Client::commit_unchecked`.

### 5.2 Configuration

Env vars (no config file in phase 1):

- `KG_SERVER_BIND` — default `127.0.0.1:9000`
- `NEO4J_URI` — default `bolt://localhost:7687`
- `NEO4J_USER` — default `neo4j`
- `NEO4J_PASSWORD` — required
- `KG_CORS_ORIGINS` — comma-separated, default `http://localhost:5173` (Vite dev)

### 5.3 Crate structure

```rust
// kg-server/src/main.rs
#[tokio::main]
async fn main() {
    let cfg = Config::from_env();
    let client = ClientBuilder::new(&cfg.neo4j_uri)
        .auth(basic(&cfg.neo4j_user, &cfg.neo4j_password))
        .build().await.unwrap();
    let state = AppState { client: Arc::new(client) };
    let app = Router::new()
        .route("/health",  get(routes::health))
        .route("/query",   post(routes::query))
        .route("/commit",  post(routes::commit))
        .with_state(state)
        .layer(CorsLayer::permissive());  // dev-only
    axum::serve(listener, app).await.unwrap();
}
```

### 5.4 Error handling

Server errors map to JSON `{"error":{"code":"<code>","message":"<msg>"}}` with
HTTP 4xx/5xx:

- `Neo4jError::Tx { code, message }` → 422 with the code in the body
- `Neo4jError::Transport(_)` → 502
- `Neo4jError::Auth(_)` → 500 (misconfig; auth is server-internal in phase 1)
- `Neo4jError::Core(_)` → 400
- Anything else → 500

## 6. webui SPA

### 6.1 Tech stack

| Concern        | Library                                |
|----------------|----------------------------------------|
| Bundler        | Vite                                   |
| Language       | TypeScript (strict)                    |
| Framework      | React 18                               |
| State          | Zustand (single store)                 |
| Graph render   | Cytoscape.js + cytoscape-cose-bilkent  |
| Styling        | CSS Modules (no Tailwind in phase 1)   |
| Fetch wrapper  | native fetch + small typed wrapper     |
| WASM           | kg-core-wasm via wasm-pack             |
| Tests          | Vitest + React Testing Library         |
| E2E (optional) | Playwright (one smoke test only)       |

### 6.2 Layout

```
+--------------------------------------------------------------------+
|  Toolbar: [Refresh] [Layout v] [Commit (3)] [Discard]              |
+--------------------------------------------------------+-----------+
|                                                        |           |
|                    Cytoscape canvas                    | Inspector |
|                                                        |  panel    |
|                  (pan / zoom / select)                 |           |
|                                                        |  Selected |
|                                                        |  node /   |
|                                                        |  rel /    |
|                                                        |  empty    |
|                                                        |           |
+--------------------------------------------------------+-----------+
|  Cypher box:  [ MATCH (n) RETURN n LIMIT 50          ▶ ] [Run]     |
+--------------------------------------------------------------------+
|  Results table (rows from non-graph-returning queries)             |
+--------------------------------------------------------------------+
```

Right pane width is resizable. Bottom Cypher box is collapsible.

### 6.3 State model (Zustand store)

```typescript
type StoreState = {
  // graph
  nodes: Map<NodeId, NodeView>;             // server-side nodes currently in view
  rels:  Map<RelId,  RelView>;
  pendingNodes: Map<LocalId, NodeView>;     // staged but not yet committed
  pendingRels:  Map<LocalId, RelView>;

  // selection
  selectedKind: "node" | "rel" | "none";
  selectedId:   string | null;              // serialized id or local-id key

  // staging (one UoW at a time)
  uow: WasmUow | null;                       // null after a commit
  pendingOpsCount: number;                   // mirrored from uow for badge

  // query
  cypherText: string;
  queryResultRows: Array<Record<string, unknown>>;
  queryError: string | null;

  // canvas
  layoutName: "cose" | "dagre" | "grid" | "preset";

  // actions
  stageCreateNode(labels: string[], props: PropsMap): void;
  stageUpdateNode(id: NodeId, patch: PatchMap): void;
  // ... one per UoW method
  commit(): Promise<void>;
  discard(): void;
  runCypher(text: string): Promise<void>;
  refresh(): Promise<void>;
};
```

### 6.4 Interaction map

| Gesture                           | Effect                                                |
|-----------------------------------|-------------------------------------------------------|
| Click node                        | Inspector shows node props (read-only until Edit)     |
| Click rel                         | Inspector shows rel props                             |
| Right-click empty canvas          | Context menu → `Add node` → label picker → stages CreateNode |
| Drag from node A onto node B      | Rel-type picker → stages CreateRel                    |
| Inspector → Edit                  | Fields become editable; Save = stages UpdateNode/Rel  |
| Inspector → Delete                | Confirm modal → stages DeleteNode (Detach) / DeleteRel|
| Toolbar `Commit (N)`              | Calls `uow.emit()` → POSTs to /commit → refreshes canvas |
| Toolbar `Discard`                 | Drops the UoW, removes pending nodes/rels from view   |
| Cypher box Run                    | POSTs to /query → merges returned graph entities into view; non-graph rows go to ResultsTable |
| Toolbar `Refresh`                 | `MATCH (n) RETURN n LIMIT $cap` + edges; resets view  |

### 6.5 Visual feedback for pending state

Pending nodes/rels render with a dashed outline and 60% opacity. The
toolbar's `Commit (N)` button shows the count of staged ops; clicking it
flushes them.

### 6.6 First-load behaviour

On mount, webui calls `/health` (toast on failure) then issues a default
`MATCH (n) RETURN n LIMIT 50` plus a follow-up MATCH for incident rels, to
populate the initial canvas.

## 7. Commit semantics

The browser stages mutations into a single `WasmUow`. There is at most one
in-flight UoW per session. `Commit` does:

1. `WasmSchemaRegistry.validate(uow)` if a registry is configured (phase 1
   leaves this off by default; schemas can be declared in webui code or
   omitted entirely).
2. `uow.emit()` → `EmitOutputJs`.
3. POST to `/commit`. On 200, drop the UoW, run `Refresh`.
4. On 4xx/5xx, show a toast with the error message; UoW retained so the user
   can fix and retry.

There is no client-side optimistic rendering of the result other than the
already-rendered pending entities. After successful commit, the canvas is
refreshed from the server so server-assigned ids replace local ones.

## 8. Cypher query box

Users type Cypher into a textarea. On Run:

- POST to `/query` with body `{cypher, params: {}}`.
- The response `rows` array is inspected:
  - Rows whose values include nodes/rels (server emits these as typed JSON
    objects with `id`, `labels`/`type`, `properties`) are merged into the
    canvas.
  - All rows go into the ResultsTable for inspection.
- A `LIMIT 200` is silently appended if the query has no top-level LIMIT
  clause and starts with `MATCH`. Other queries are sent as-is.

Phase 1 does not support write-Cypher through the query box (no `CREATE`,
`SET`, `MERGE` from the textarea). Detection is a simple keyword scan; if
the query contains those keywords the UI refuses with a toast directing the
user to use the canvas editor instead. (This is a defensive default, not
real security — there is no auth in phase 1.)

## 9. Testing strategy

### Tier 1 — kg-core-wasm

A small wasm-bindgen-test suite asserts the JS-facing bindings produce the
same `EmitOutput` shape as the Rust crate. Reuses the `emit_snapshots`
fixtures from phase-0.

### Tier 2 — kg-server

Integration tests in `crates/kg-server/tests/server.rs` that spin up a
testcontainers Neo4j AND the axum server, then drive it through HTTP:

- `/health` returns OK.
- POST `/commit` with a known EmitOutput JSON creates the expected nodes +
  rels (verified via a follow-up `/query`).
- POST `/query` returns rows.
- `/commit` with an invalid Cypher body returns 422 with the error code in
  the body.

### Tier 3 — webui

- Vitest unit tests for the Zustand store actions and the `kg/uow.ts` JS
  glue. WASM is loaded in `vitest.setup.ts` via wasm-pack node target.
- One Playwright smoke test: start the dev server + kg-server (orchestrated
  by a script), open the page, drag-to-create a rel, commit, assert via
  /query that it landed.

### Tier 4 — Manual

`docs/manual-ui-smoke.md` checklist for one human run per release.

## 10. Build / run

Local dev:

```
make neo4j-up
make kg-server-run          # cargo run -p kg-server
make webui-dev              # cd webui && npm run dev
open http://localhost:5173
```

Production build:

```
wasm-pack build crates/kg-core-wasm --target web --out-dir ../../webui/pkg
cd webui && npm run build      # writes to webui/dist
cargo build --release -p kg-server
```

`kg-server` can serve the built `webui/dist` directly if `KG_SERVE_UI=/path/to/dist` is set, so a single binary deploys both.

## 11. CI additions

New jobs added to `.github/workflows/ci.yml`:

- `wasm-bindings`: builds `kg-core-wasm` via wasm-pack, runs wasm-bindgen-tests.
- `kg-server-test`: runs the new server integration tests against the existing Neo4j service container.
- `webui-build`: installs npm, builds the SPA, runs vitest unit tests.
- `webui-smoke`: Playwright headless run (optional — gated behind a label or kept off by default in phase 1 to keep CI minutes low).

## 12. Phase-1 Acceptance Criteria

1. SPA loads at `localhost:5173`, connects to `kg-server`, and shows a
   default `MATCH (n) RETURN n LIMIT 50` result on the canvas.
2. User can create a node by right-clicking empty canvas, picking a label,
   and entering one prop. Committing causes the node to persist in Neo4j and
   re-appear on next refresh with a server id.
3. User can create a rel by dragging from node A onto node B, picking a
   type, and committing.
4. User can click a node, edit one prop in the inspector, save, commit, and
   the change is reflected on refresh.
5. User can delete a node (Detach) from the inspector and commit; the node
   and its incident rels are gone on refresh.
6. User can run a `MATCH (n:Person) RETURN n LIMIT 10` query in the Cypher
   box; the canvas shows only those Person nodes.
7. All four kg-server integration tests pass against `neo4j:5-community`
   via testcontainers.
8. The phase-0 native and wasm test suites continue to pass.

## 13. Out of Scope / Phase 2+

- Authentication / multi-user.
- Real-time collaborative editing.
- Undo/redo history beyond Discard.
- Schema editing UI.
- Custom node/edge styling per label/type from the UI.
- Touch / mobile.
- Offline / local persistence of pending UoW across reloads.
- Importing data files via the UI.
- Saved queries / query history.
- Cytoscape extensions beyond cose-bilkent + dagre.

## 14. Open Questions

None blocking. The following will be revisited during plan writing if they
become decisions:

- Whether the Cypher box should expose a "write mode" toggle (gated by a UI
  setting) for power users in phase 1.
- Whether to ship the WASM bundle from npm or as a sibling directory
  (current plan: sibling directory under `webui/pkg`).
- Whether kg-server's `/query` should stream results for very large result
  sets (current plan: buffered).
