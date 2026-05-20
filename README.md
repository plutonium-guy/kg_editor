# kg_editor

Browser-based knowledge-graph editor for SMEs (no Cypher knowledge required) + MCP server so AI agents can drive the same graph. Backed by Neo4j 5.

## Stack

- **Go** backend (`cmd/kg-server`) — chi + neo4j-go-driver/v5; serves the REST API the webui consumes.
- **Go** MCP server (`cmd/kg-mcp`) — mcp-go over stdio; calls kg-server.
- **TypeScript + React** webui — Vite, react-router, react-hook-form + zod, @tanstack/react-query, Cytoscape.
- **YAML schema** (`kg-schema.yaml`) — single source of truth for entity types, relationships, validation rules. Loaded once at server startup.

Phase 0–2 Rust implementation lives in git history (`phase-0/kg-editor`, `phase-1/ui`, `phase-2/sme-ai` branches). Replaced by Go in phase 3 for iteration speed + Neo4j driver maturity.

## Layout

```
.
├── cmd/
│   ├── kg-server/       # HTTP API binary
│   └── kg-mcp/          # MCP stdio binary
├── internal/
│   ├── api/             # chi handlers
│   ├── schema/          # YAML loader + validation
│   ├── store/           # neo4j driver wrapper
│   └── mcp/             # MCP tools + REST client
├── webui/               # Vite + React + TS frontend
├── kg-schema.yaml       # schema source of truth
└── docs/                # design / plan / smoke docs
```

## Run

```
make neo4j-up
make kg-server-run      # starts on :9000 — needs KG_SCHEMA + NEO4J_PASSWORD env, see Makefile
make webui-dev          # starts Vite on :5173
# AI side, separate terminal:
make kg-mcp-run         # stdio MCP — wire into Claude per docs/manual-mcp-smoke.md
```

## Test

```
go test ./internal/schema/... ./internal/mcp/...                                   # unit, no Neo4j
NEO4J_URI=bolt://localhost:7687 NEO4J_PASSWORD=testtest \
  go test -tags=integration ./internal/store/... ./internal/api/...                # integration
cd webui && npm test                                                                # vitest
cd webui && npm run e2e                                                             # playwright (kg-server must be running)
```

## Docs

- [Phase 3 design](docs/superpowers/specs/2026-05-20-phase-3-go-rewrite-design.md)
- [Manual UI smoke](docs/manual-ui-smoke.md)
- [Manual MCP smoke](docs/manual-mcp-smoke.md)

## License

MIT OR Apache-2.0
