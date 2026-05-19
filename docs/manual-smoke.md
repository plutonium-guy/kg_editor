# Manual WASM smoke (one-time per release)

## Prereqs
- Docker (Colima: set `DOCKER_HOST=unix:///$HOME/.colima/default/docker.sock`)
- `wasm-pack` installed (`cargo install wasm-pack`)
- A static file server (`python3 -m http.server`)

## Steps
1. `make neo4j-up`
2. Open `cypher-shell` once. Neo4j 5 Community blocks browser fetch without CORS — set the relevant env when launching Neo4j or run a CORS proxy. Example:
   ```
   docker run -d --name kg-neo4j -p 7687:7687 -p 7474:7474 \
     -e NEO4J_AUTH=neo4j/test \
     -e NEO4J_dbms_security_http__auth__allowlist=* \
     neo4j:5-community
   ```
3. `cd examples/wasm_crud && wasm-pack build --target web`
4. `python3 -m http.server` then open <http://localhost:8000/index.html>
5. Open browser console; expect `committed` logged and no errors.
6. In `cypher-shell`: `MATCH (a:Person)-[:KNOWS]->(b:Person) RETURN a.name, b.name;` — should return Alice, Bob.

Record result here:

| Date       | Engineer | Browser            | Pass/Fail | Notes |
|------------|----------|--------------------|-----------|-------|
| 2026-05-20 | amiyamandal | Chrome (headless, wasm-pack 0.13) | PASS | Executed via `wasm-pack test --headless --chrome crates/kg-neo4j --no-default-features --features wasm`; 2/2 tests passed (`recording_client_captures_request`, `recording_client_canned_body_passes_through`). Browser-to-Neo4j live run still pending. |
