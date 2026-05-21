// Package mcp implements the kg-mcp tools. They translate MCP tool calls
// into HTTP requests against kg-server.
package mcp

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strconv"
	"time"
)

// Client is a thin wrapper around kg-server's REST surface.
type Client struct {
	Base string
	HTTP *http.Client
}

// NewClient returns a Client targeting base (e.g. "http://localhost:9000").
func NewClient(base string) *Client {
	return &Client{Base: base, HTTP: &http.Client{Timeout: 30 * time.Second}}
}

func (c *Client) doJSON(ctx context.Context, method, path string, body any) ([]byte, error) {
	var rdr io.Reader
	if body != nil {
		b, err := json.Marshal(body)
		if err != nil {
			return nil, err
		}
		rdr = bytes.NewReader(b)
	}
	req, err := http.NewRequestWithContext(ctx, method, c.Base+path, rdr)
	if err != nil {
		return nil, err
	}
	if body != nil {
		req.Header.Set("content-type", "application/json")
	}
	resp, err := c.HTTP.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	data, _ := io.ReadAll(resp.Body)
	if resp.StatusCode/100 != 2 {
		return nil, fmt.Errorf("kg-server %s %s: %d %s", method, path, resp.StatusCode, string(data))
	}
	return data, nil
}

// GetSchema fetches the kg-server schema (labels, props, rels).
func (c *Client) GetSchema(ctx context.Context) (json.RawMessage, error) {
	return c.doJSON(ctx, http.MethodGet, "/schema", nil)
}

// CreateEntity creates a new node with the given label and properties.
func (c *Client) CreateEntity(ctx context.Context, label string, props map[string]any) (json.RawMessage, error) {
	return c.doJSON(ctx, http.MethodPost, "/entities", map[string]any{"label": label, "props": props})
}

// UpdateEntity updates a node's properties by id.
func (c *Client) UpdateEntity(ctx context.Context, id int64, set map[string]any, unset []string) (json.RawMessage, error) {
	return c.doJSON(ctx, http.MethodPut, fmt.Sprintf("/entities/%d", id), map[string]any{"set": set, "unset": unset})
}

// DeleteEntity deletes a node by id. Pass cascade=true to DETACH DELETE.
func (c *Client) DeleteEntity(ctx context.Context, id int64, cascade bool) (json.RawMessage, error) {
	return c.doJSON(ctx, http.MethodDelete, fmt.Sprintf("/entities/%d?cascade=%t", id, cascade), nil)
}

// LinkEntities creates a typed relationship between two nodes.
func (c *Client) LinkEntities(ctx context.Context, ty string, startID, endID int64, props map[string]any) (json.RawMessage, error) {
	return c.doJSON(ctx, http.MethodPost, "/links", map[string]any{"type": ty, "start_id": startID, "end_id": endID, "props": props})
}

// UnlinkEntities deletes a relationship by id.
func (c *Client) UnlinkEntities(ctx context.Context, relID int64) (json.RawMessage, error) {
	return c.doJSON(ctx, http.MethodDelete, fmt.Sprintf("/links/%d", relID), nil)
}

// FindEntities lists or searches entities with optional label/q/limit filters.
func (c *Client) FindEntities(ctx context.Context, label, q string, limit int) (json.RawMessage, error) {
	v := url.Values{}
	if label != "" {
		v.Set("label", label)
	}
	if q != "" {
		v.Set("q", q)
	}
	if limit > 0 {
		v.Set("limit", strconv.Itoa(limit))
	}
	path := "/entities"
	if len(v) > 0 {
		path += "?" + v.Encode()
	}
	return c.doJSON(ctx, http.MethodGet, path, nil)
}

// PutSchema replaces the entire schema. Body must be a pre-serialised SchemaFile JSON.
func (c *Client) PutSchema(ctx context.Context, body json.RawMessage) (json.RawMessage, error) {
	return c.doRaw(ctx, http.MethodPut, "/schema", body)
}

// ReloadSchema re-loads the schema from Neo4j into the server cache.
func (c *Client) ReloadSchema(ctx context.Context) (json.RawMessage, error) {
	return c.doJSON(ctx, http.MethodPost, "/schema/reload", nil)
}

// CreateSchemaNode adds a new node label definition.
func (c *Client) CreateSchemaNode(ctx context.Context, name string, def map[string]any) (json.RawMessage, error) {
	return c.doJSON(ctx, http.MethodPost, "/schema/nodes", map[string]any{"name": name, "def": def})
}

// UpdateSchemaNode replaces an existing label's definition.
func (c *Client) UpdateSchemaNode(ctx context.Context, label string, def map[string]any) (json.RawMessage, error) {
	return c.doJSON(ctx, http.MethodPut, fmt.Sprintf("/schema/nodes/%s", label), def)
}

// DeleteSchemaNode removes a label from the schema.
func (c *Client) DeleteSchemaNode(ctx context.Context, label string) (json.RawMessage, error) {
	return c.doJSON(ctx, http.MethodDelete, fmt.Sprintf("/schema/nodes/%s", label), nil)
}

// CreateSchemaRel adds a new relationship type definition.
func (c *Client) CreateSchemaRel(ctx context.Context, ty string, def map[string]any) (json.RawMessage, error) {
	return c.doJSON(ctx, http.MethodPost, "/schema/rels", map[string]any{"type": ty, "def": def})
}

// UpdateSchemaRel replaces an existing rel type's definition.
func (c *Client) UpdateSchemaRel(ctx context.Context, ty string, def map[string]any) (json.RawMessage, error) {
	return c.doJSON(ctx, http.MethodPut, fmt.Sprintf("/schema/rels/%s", ty), def)
}

// DeleteSchemaRel removes a rel type from the schema.
func (c *Client) DeleteSchemaRel(ctx context.Context, ty string) (json.RawMessage, error) {
	return c.doJSON(ctx, http.MethodDelete, fmt.Sprintf("/schema/rels/%s", ty), nil)
}

// doRaw is like doJSON but body is already serialised JSON (json.RawMessage).
func (c *Client) doRaw(ctx context.Context, method, path string, body json.RawMessage) (json.RawMessage, error) {
	var rdr io.Reader
	if len(body) > 0 {
		rdr = bytes.NewReader(body)
	}
	req, err := http.NewRequestWithContext(ctx, method, c.Base+path, rdr)
	if err != nil {
		return nil, err
	}
	if len(body) > 0 {
		req.Header.Set("content-type", "application/json")
	}
	resp, err := c.HTTP.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	data, _ := io.ReadAll(resp.Body)
	if resp.StatusCode/100 != 2 {
		return nil, fmt.Errorf("kg-server %s %s: %d %s", method, path, resp.StatusCode, string(data))
	}
	return data, nil
}
