# Phase 3 — Go Backend Rewrite + TS Frontend (Scrap Rust)

**Date:** 2026-05-20
**Status:** Design — approved (autonomous execution authorized)
**Owner:** plutonium-guy
**Replaces:** all phase 0-2 Rust crates

## 1. Purpose

Replace the Rust workspace (`kg-core`, `kg-neo4j`, `kg-core-wasm`, `kg-server`, `kg-mcp`, `examples`) with a Go backend that preserves the REST contract the React frontend already speaks. Iteration speed and the maturity of the official Neo4j Go driver are the primary motivations. The TS webui (phase-2/sme-ai) is kept as-is — it talks REST, the REST surface is contract-locked.

The phase-1 raw-Cypher panel and the phase-1 `/query` + `/commit` endpoints are dropped. Phase-2 introduced superset endpoints (`/entities`, `/links`, `/search`) that cover every UI need. The new server only ships those.

## 2. Architecture

```
kg_editor/
├── kg-schema.yaml          # KEEP — single source of truth for schema
├── go.mod                  # NEW — Go module root
├── go.sum
├── cmd/
│   ├── kg-server/
│   │   └── main.go         # HTTP server entry point
│   └── kg-mcp/
│       └── main.go         # MCP stdio server entry point
├── internal/
│   ├── schema/             # YAML loader + FieldSpec/FieldType
│   │   ├── schema.go
│   │   └── schema_test.go
│   ├── store/              # neo4j driver wrapper + queries
│   │   ├── store.go
│   │   └── store_test.go   # testcontainers-go
│   ├── api/                # HTTP handlers
│   │   ├── server.go       # router + middleware
│   │   ├── entities.go
│   │   ├── links.go
│   │   ├── search.go
│   │   ├── schema.go
│   │   ├── health.go
│   │   ├── errors.go
│   │   └── *_test.go
│   └── mcp/                # MCP tool implementations
│       ├── tools.go
│       └── tools_test.go
├── webui/                  # KEEP — TS+React, no changes needed
├── docs/                   # KEEP
├── Makefile                # rewritten for Go targets
├── .github/workflows/ci.yml # rewritten for Go + webui jobs
└── README.md               # updated
```

## 3. Go stack

| Concern | Library | Why |
|---|---|---|
| HTTP router | `github.com/go-chi/chi/v5` | idiomatic, lightweight, stdlib-compatible |
| Neo4j driver | `github.com/neo4j/neo4j-go-driver/v5` | official, mature |
| YAML | `gopkg.in/yaml.v3` | canonical |
| MCP | `github.com/mark3labs/mcp-go` | most active community Go MCP SDK |
| CORS | `github.com/go-chi/cors` | drop-in chi middleware |
| Testing | stdlib + `github.com/stretchr/testify` | assertions only |
| Test containers | `github.com/testcontainers/testcontainers-go` | parity with phase 0-2 |
| Logging | stdlib `log/slog` | stdlib, structured |

## 4. REST contract (unchanged from phase 2)

- `GET  /health` → `{status, neo4j}`
- `GET  /schema` → `SchemaFile`
- `GET  /entities?label=&q=&limit=` → `[EntityDetail]`
- `GET  /entities/:id` → `EntityDetail`
- `POST /entities` → `{id, label, props}`
- `PUT  /entities/:id` → `{ok}`
- `DELETE /entities/:id?cascade=` → `{ok}`
- `POST /links` → `{id}`
- `DELETE /links/:id` → `{ok}`
- `GET  /search?q=&limit=` → `[EntityDetail]`

JSON shapes match phase 2 exactly. Frontend untouched.

## 5. MCP tools (unchanged from phase 2)

`get_schema`, `create_entity`, `update_entity`, `delete_entity`, `link_entities`, `unlink_entities`, `find_entities`. All call `kg-server` via HTTP. `KG_SERVER_URL` env var.

## 6. Schema types in Go

```go
type FieldType struct {
    Type   string   `yaml:"type"   json:"type"`             // string|int|float|bool|date|date_time|enum|ref
    Values []string `yaml:"values,omitempty" json:"values,omitempty"`
    Label  string   `yaml:"label,omitempty"  json:"label,omitempty"`
}

type FieldSpec struct {
    Name     string      `yaml:"name"             json:"name"`
    FieldType `yaml:",inline" json:",inline"`
    Required bool        `yaml:"required,omitempty" json:"required,omitempty"`
    Unique   bool        `yaml:"unique,omitempty"   json:"unique,omitempty"`
    Default  any         `yaml:"default,omitempty"  json:"default,omitempty"`
}

type NodeDef struct {
    Description string      `yaml:"description,omitempty" json:"description,omitempty"`
    Props       []FieldSpec `yaml:"props"                 json:"props"`
    Indexes     [][]string  `yaml:"indexes,omitempty"     json:"indexes,omitempty"`
}

type RelDef struct {
    Endpoints   [][2]string `yaml:"endpoints"             json:"endpoints"`
    Cardinality string      `yaml:"cardinality,omitempty" json:"cardinality"`
    Props       []FieldSpec `yaml:"props,omitempty"       json:"props"`
}

type SchemaFile struct {
    Nodes map[string]NodeDef `yaml:"nodes" json:"nodes"`
    Rels  map[string]RelDef  `yaml:"rels"  json:"rels"`
}
```

JSON output matches phase 2's `SchemaFile` shape exactly — webui consumes without changes.

## 7. Validation

Same rules as phase 2:
- Required field present
- Field type matches (string/int/float/bool/enum string in `values`/ref is i64)
- Rel endpoints in `allowed_endpoints`
- Unknown label / unknown rel type → 400

Errors return `{"error":{"code":"400.validation","message":"..."}}` body to keep contract.

## 8. Testing

- **schema/schema_test.go** — YAML parse + validation cross-refs.
- **store/store_test.go** — testcontainers-go spins Neo4j; CRUD smoke.
- **api/*_test.go** — `httptest.Server` + chi router; full REST surface tested against a real Neo4j (one container shared across the suite via `TestMain`).
- **mcp/tools_test.go** — point at a mock HTTP server, exercise each tool.

## 9. CI

`.github/workflows/ci.yml` jobs:
- `go-build` + `go-vet` + `golangci-lint`
- `go-test-unit` (no Neo4j)
- `go-test-integration` (services: neo4j)
- `webui-build` + `webui-test`
- `webui-e2e` (services: neo4j + Go server built+launched)

## 10. Migration plan

1. Branch `phase-3/go-rewrite` off main.
2. Delete `crates/`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `examples/`, `webui/pkg/`.
3. Initialize Go module + scaffold directories.
4. Implement schema package.
5. Implement store package with testcontainers.
6. Implement api package (health + schema + entities + links + search).
7. Implement mcp package + binary.
8. Rewrite Makefile and CI.
9. Update README + manual smoke docs.
10. Run webui against the new Go server end-to-end — verify zero TS changes needed.

## 11. Acceptance

1. `go build ./...` succeeds.
2. `go test ./...` passes (unit + integration).
3. `make neo4j-up && make kg-server-run` boots the Go server.
4. webui at `localhost:5173` (no changes) operates exactly as in phase 2: create/edit/link/delete via forms, graph view, search.
5. `kg-mcp` binary completes the same MCP smoke conversation from phase 2.
6. Repo contains zero Rust source files (`grep -r --include='*.rs' . | wc -l` == 0; `find . -name 'Cargo.toml'` empty).

## 12. Open Questions

None. Authorized for autonomous execution.
