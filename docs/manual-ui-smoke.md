# Manual UI smoke (one-time per release)

## Prereqs
- Docker (Colima: `export DOCKER_HOST=unix://${HOME}/.colima/default/docker.sock`)
- Node 20+, npm
- wasm-pack (`cargo install wasm-pack`)

## Steps
1. `make neo4j-up`
2. In another terminal: `NEO4J_PASSWORD=test cargo run -p kg-server`
3. In another terminal: `make wasm-build && cd webui && npm install && npm run dev`
4. Open <http://localhost:5173>. Browser console should show no errors.
5. **Create node:** right-click empty canvas → label "Person", name "Alice" → toolbar shows `Commit (1)` → click Commit. After refresh, Alice node should appear with a server id.
6. **Create rel:** create another Person "Bob" the same way. Hold Shift + drag from Alice onto Bob. Type "KNOWS". Commit. Refresh shows the edge.
7. **Edit node:** click Alice. Inspector shows props. Edit → change name to "Alicia" → Save → Commit. Refresh shows name updated.
8. **Delete rel:** click the KNOWS edge → Delete → Commit. Refresh shows no edge.
9. **Cypher query:** clear the cypher box, paste `MATCH (n:Person) RETURN n.name AS name`. Click Run. ResultsTable shows Alicia + Bob rows.
10. **Delete node:** click Bob → Delete → Commit. Refresh shows only Alicia.

Record result here:

| Date       | Engineer | Browser     | Pass/Fail | Notes |
|------------|----------|-------------|-----------|-------|
| YYYY-MM-DD | name     | Chrome 140  |           |       |
