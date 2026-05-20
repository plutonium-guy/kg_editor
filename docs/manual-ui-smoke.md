# Manual UI smoke (one-time per release)

## Prereqs
- Docker (Colima: `export DOCKER_HOST=unix://${HOME}/.colima/default/docker.sock`)
- Node 20+, npm
- `make neo4j-up` running

## Steps

1. Start backend with schema:
   ```
   KG_SCHEMA=$(pwd)/kg-schema.yaml NEO4J_PASSWORD=testtest cargo run -p kg-server
   ```
2. Start webui:
   ```
   cd webui && npm install && npm run dev
   ```
3. Open <http://localhost:5173>. You should land on `/browse` with a sidebar listing the labels from `kg-schema.yaml` (Person, Company, Skill).

4. **Create entity:** Click "+ Add Person" in the sidebar. Drawer opens with a form for name, age, role, email, joined. Fill `name=Alice` and `role=engineer` (enum dropdown). Click "Stage". The orange pending bar shows "1 pending change". Click "Commit all". Alice appears in the Person list.

5. **Edit entity:** Click Alice's name. Inspector loads. Click "Edit", change role from "engineer" to "manager", click "Stage update". Commit. Refresh shows the change.

6. **Link entity:** On Alice's page, click "+ Link". Picker shows allowed types (KNOWS, WORKS_AT, HAS_SKILL). Pick KNOWS. Type "Alice" in the target picker (or any Person name). Select target. Fill `since=2024`. Click "Stage link". Commit. The KNOWS rel appears under Relationships.

7. **Delete entity:** Click another Person. Click "Delete entity". Pending. Commit. Entity is gone from sidebar.

8. **Graph view:** Click "Graph" in nav. See the graph with label-colored nodes. Tick a label to filter.

9. **Search:** Click "Search" in nav. Type a name. Matching entities link to their detail pages.

| Date       | Engineer | Browser     | Pass/Fail | Notes |
|------------|----------|-------------|-----------|-------|
| YYYY-MM-DD | name     | Chrome 140  |           |       |
