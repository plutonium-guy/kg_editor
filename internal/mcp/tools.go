package mcp

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/mark3labs/mcp-go/mcp"
	"github.com/mark3labs/mcp-go/server"
)

// textResult wraps a raw JSON byte slice as an MCP text content result.
func textResult(b []byte) *mcp.CallToolResult {
	return &mcp.CallToolResult{
		Content: []mcp.Content{mcp.TextContent{Type: "text", Text: string(b)}},
	}
}

// RegisterTools wires the 15 kg-editor tools on s. All tools forward their
// calls to c (kg-server REST) and return the raw JSON response as text.
func RegisterTools(s *server.MCPServer, c *Client) {
	// get_schema -------------------------------------------------------
	s.AddTool(
		mcp.NewTool("get_schema",
			mcp.WithDescription("Get the knowledge-graph schema (labels, props, rels)."),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			b, err := c.GetSchema(ctx)
			if err != nil {
				return nil, err
			}
			return &mcp.CallToolResult{
				Content: []mcp.Content{mcp.TextContent{Type: "text", Text: string(b)}},
			}, nil
		},
	)

	// create_entity ----------------------------------------------------
	s.AddTool(
		mcp.NewTool("create_entity",
			mcp.WithDescription("Create a node with the given label and props."),
			mcp.WithString("label", mcp.Required(), mcp.Description("Node label")),
			mcp.WithObject("props", mcp.Description("Property map (optional)")),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			label := req.GetString("label", "")
			props, _ := req.GetArguments()["props"].(map[string]any)
			if props == nil {
				props = map[string]any{}
			}
			b, err := c.CreateEntity(ctx, label, props)
			if err != nil {
				return nil, err
			}
			return &mcp.CallToolResult{
				Content: []mcp.Content{mcp.TextContent{Type: "text", Text: string(b)}},
			}, nil
		},
	)

	// update_entity ----------------------------------------------------
	s.AddTool(
		mcp.NewTool("update_entity",
			mcp.WithDescription("Update a node's properties."),
			mcp.WithNumber("id", mcp.Required(), mcp.Description("Node id")),
			mcp.WithObject("set", mcp.Description("Properties to set")),
			mcp.WithArray("unset", mcp.Description("Property keys to remove")),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			id := int64(req.GetFloat("id", 0))
			set, _ := req.GetArguments()["set"].(map[string]any)
			unset := req.GetStringSlice("unset", nil)
			b, err := c.UpdateEntity(ctx, id, set, unset)
			if err != nil {
				return nil, err
			}
			return &mcp.CallToolResult{
				Content: []mcp.Content{mcp.TextContent{Type: "text", Text: string(b)}},
			}, nil
		},
	)

	// delete_entity ----------------------------------------------------
	s.AddTool(
		mcp.NewTool("delete_entity",
			mcp.WithDescription("Delete a node by id. Optionally cascade (DETACH DELETE)."),
			mcp.WithNumber("id", mcp.Required(), mcp.Description("Node id")),
			mcp.WithBoolean("cascade", mcp.Description("DETACH DELETE if true")),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			id := int64(req.GetFloat("id", 0))
			cascade := req.GetBool("cascade", false)
			b, err := c.DeleteEntity(ctx, id, cascade)
			if err != nil {
				return nil, err
			}
			return &mcp.CallToolResult{
				Content: []mcp.Content{mcp.TextContent{Type: "text", Text: string(b)}},
			}, nil
		},
	)

	// link_entities ----------------------------------------------------
	s.AddTool(
		mcp.NewTool("link_entities",
			mcp.WithDescription("Create a typed relationship between two nodes."),
			mcp.WithString("type", mcp.Required(), mcp.Description("Relationship type")),
			mcp.WithNumber("start_id", mcp.Required(), mcp.Description("Start node id")),
			mcp.WithNumber("end_id", mcp.Required(), mcp.Description("End node id")),
			mcp.WithObject("props", mcp.Description("Relationship properties (optional)")),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			ty := req.GetString("type", "")
			startID := int64(req.GetFloat("start_id", 0))
			endID := int64(req.GetFloat("end_id", 0))
			props, _ := req.GetArguments()["props"].(map[string]any)
			if props == nil {
				props = map[string]any{}
			}
			b, err := c.LinkEntities(ctx, ty, startID, endID, props)
			if err != nil {
				return nil, err
			}
			return &mcp.CallToolResult{
				Content: []mcp.Content{mcp.TextContent{Type: "text", Text: string(b)}},
			}, nil
		},
	)

	// unlink_entities --------------------------------------------------
	s.AddTool(
		mcp.NewTool("unlink_entities",
			mcp.WithDescription("Delete a relationship by id."),
			mcp.WithNumber("rel_id", mcp.Required(), mcp.Description("Relationship id")),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			relID := int64(req.GetFloat("rel_id", 0))
			b, err := c.UnlinkEntities(ctx, relID)
			if err != nil {
				return nil, err
			}
			return &mcp.CallToolResult{
				Content: []mcp.Content{mcp.TextContent{Type: "text", Text: string(b)}},
			}, nil
		},
	)

	// find_entities ----------------------------------------------------
	s.AddTool(
		mcp.NewTool("find_entities",
			mcp.WithDescription("List or search entities. Optional label filter, optional substring q, optional limit."),
			mcp.WithString("label", mcp.Description("Filter by label (optional)")),
			mcp.WithString("q", mcp.Description("Substring search across props (optional)")),
			mcp.WithNumber("limit", mcp.Description("Max results (optional)")),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			label := req.GetString("label", "")
			q := req.GetString("q", "")
			limit := req.GetInt("limit", 0)
			b, err := c.FindEntities(ctx, label, q, limit)
			if err != nil {
				return nil, err
			}
			return &mcp.CallToolResult{
				Content: []mcp.Content{mcp.TextContent{Type: "text", Text: string(b)}},
			}, nil
		},
	)

	// put_schema -------------------------------------------------------
	s.AddTool(
		mcp.NewTool("put_schema",
			mcp.WithDescription("Replace the entire schema. Body must be the full {nodes, rels} SchemaFile JSON."),
			mcp.WithObject("schema", mcp.Required()),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			args := req.GetArguments()
			s, ok := args["schema"]
			if !ok {
				return nil, fmt.Errorf("missing schema")
			}
			body, err := json.Marshal(s)
			if err != nil {
				return nil, err
			}
			b, err := c.PutSchema(ctx, body)
			if err != nil {
				return nil, err
			}
			return textResult(b), nil
		},
	)

	// reload_schema ----------------------------------------------------
	s.AddTool(
		mcp.NewTool("reload_schema",
			mcp.WithDescription("Re-load the schema from Neo4j into the server cache."),
		),
		func(ctx context.Context, _ mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			b, err := c.ReloadSchema(ctx)
			if err != nil {
				return nil, err
			}
			return textResult(b), nil
		},
	)

	// create_schema_label ----------------------------------------------
	s.AddTool(
		mcp.NewTool("create_schema_label",
			mcp.WithDescription("Add a new node label. `def` is a NodeDef object: {description, props:[{name,type,values?,label?,required?,unique?}], indexes:[[colName,...]]}."),
			mcp.WithString("name", mcp.Required()),
			mcp.WithObject("def", mcp.Required()),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			args := req.GetArguments()
			name, _ := args["name"].(string)
			def, _ := args["def"].(map[string]any)
			if def == nil {
				def = map[string]any{}
			}
			b, err := c.CreateSchemaNode(ctx, name, def)
			if err != nil {
				return nil, err
			}
			return textResult(b), nil
		},
	)

	// update_schema_label ----------------------------------------------
	s.AddTool(
		mcp.NewTool("update_schema_label",
			mcp.WithDescription("Replace an existing label's definition."),
			mcp.WithString("label", mcp.Required()),
			mcp.WithObject("def", mcp.Required()),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			args := req.GetArguments()
			label, _ := args["label"].(string)
			def, _ := args["def"].(map[string]any)
			if def == nil {
				def = map[string]any{}
			}
			b, err := c.UpdateSchemaNode(ctx, label, def)
			if err != nil {
				return nil, err
			}
			return textResult(b), nil
		},
	)

	// delete_schema_label ----------------------------------------------
	s.AddTool(
		mcp.NewTool("delete_schema_label",
			mcp.WithDescription("Delete a label from the schema. Existing entities with this label keep their data."),
			mcp.WithString("label", mcp.Required()),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			args := req.GetArguments()
			label, _ := args["label"].(string)
			b, err := c.DeleteSchemaNode(ctx, label)
			if err != nil {
				return nil, err
			}
			return textResult(b), nil
		},
	)

	// create_schema_rel ------------------------------------------------
	s.AddTool(
		mcp.NewTool("create_schema_rel",
			mcp.WithDescription("Add a new relationship type. `def` is a RelDef: {endpoints:[[fromLabel,toLabel],...], cardinality, props:[...]}."),
			mcp.WithString("type", mcp.Required()),
			mcp.WithObject("def", mcp.Required()),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			args := req.GetArguments()
			ty, _ := args["type"].(string)
			def, _ := args["def"].(map[string]any)
			if def == nil {
				def = map[string]any{}
			}
			b, err := c.CreateSchemaRel(ctx, ty, def)
			if err != nil {
				return nil, err
			}
			return textResult(b), nil
		},
	)

	// update_schema_rel ------------------------------------------------
	s.AddTool(
		mcp.NewTool("update_schema_rel",
			mcp.WithDescription("Replace an existing rel type's definition."),
			mcp.WithString("type", mcp.Required()),
			mcp.WithObject("def", mcp.Required()),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			args := req.GetArguments()
			ty, _ := args["type"].(string)
			def, _ := args["def"].(map[string]any)
			if def == nil {
				def = map[string]any{}
			}
			b, err := c.UpdateSchemaRel(ctx, ty, def)
			if err != nil {
				return nil, err
			}
			return textResult(b), nil
		},
	)

	// delete_schema_rel ------------------------------------------------
	s.AddTool(
		mcp.NewTool("delete_schema_rel",
			mcp.WithDescription("Delete a rel type from the schema."),
			mcp.WithString("type", mcp.Required()),
		),
		func(ctx context.Context, req mcp.CallToolRequest) (*mcp.CallToolResult, error) {
			args := req.GetArguments()
			ty, _ := args["type"].(string)
			b, err := c.DeleteSchemaRel(ctx, ty)
			if err != nil {
				return nil, err
			}
			return textResult(b), nil
		},
	)
}
