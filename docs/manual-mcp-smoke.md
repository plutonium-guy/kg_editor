# Manual MCP smoke (one-time per release)

## Prereqs
- Neo4j + kg-server running (per manual-ui-smoke.md steps 1)
- Claude Desktop or Claude Code with MCP support

## Steps

1. Add this entry to your Claude MCP config (`~/Library/Application Support/Claude/claude_desktop_config.json` for Claude Desktop, or `~/.claude/mcp.json` for Claude Code):

   ```json
   {
     "mcpServers": {
       "kg-editor": {
         "command": "go",
         "args": ["run", "./cmd/kg-mcp"],
         "cwd": "/Volumes/external_storage/kg_editor",
         "env": { "KG_SERVER_URL": "http://localhost:9000" }
       }
     }
   }
   ```

2. Restart Claude. Confirm the `kg-editor` server appears in the MCP server list.

3. In a fresh conversation, ask: "What labels are in the kg-editor schema?" — Claude should invoke `get_schema` and list Person / Company / Skill.

4. Ask: "Create a Person named Quincy, age 41, role manager." — Claude invokes `create_entity`, gets back an id, confirms.

5. Ask: "Find everyone whose name starts with Q." — `find_entities` returns Quincy.

6. Ask: "Link Quincy to Acme via WORKS_AT, role engineer." — Claude calls `find_entities` to resolve both ids, then `link_entities`.

| Date       | Engineer    | MCP client      | Pass/Fail | Notes |
|------------|-------------|-----------------|-----------|-------|
| YYYY-MM-DD | name        | Claude Code 1.x |           |       |
