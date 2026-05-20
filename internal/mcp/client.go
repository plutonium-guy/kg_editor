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
