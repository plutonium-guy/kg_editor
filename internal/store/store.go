// Package store wraps neo4j-go-driver/v5 with the subset of operations
// kg-server needs. All public functions return plain Go types (map[string]any,
// int64) so the API package can JSON-encode them directly.
package store

import (
	"context"
	"errors"
	"fmt"

	"github.com/neo4j/neo4j-go-driver/v5/neo4j"
)

// Store wraps a Neo4j driver.
type Store struct {
	driver neo4j.DriverWithContext
}

// New creates a new Store, verifying connectivity before returning.
func New(ctx context.Context, uri, user, password string) (*Store, error) {
	d, err := neo4j.NewDriverWithContext(uri, neo4j.BasicAuth(user, password, ""))
	if err != nil {
		return nil, fmt.Errorf("neo4j driver: %w", err)
	}
	if err := d.VerifyConnectivity(ctx); err != nil {
		_ = d.Close(ctx)
		return nil, fmt.Errorf("neo4j connect: %w", err)
	}
	return &Store{driver: d}, nil
}

// Close releases the underlying driver connection pool.
func (s *Store) Close(ctx context.Context) error {
	if s == nil || s.driver == nil {
		return nil
	}
	return s.driver.Close(ctx)
}

// Ping is a cheap reachability check used by /health.
func (s *Store) Ping(ctx context.Context) error {
	return s.driver.VerifyConnectivity(ctx)
}

// Entity is the row shape returned by entity reads.
type Entity struct {
	ID     int64          `json:"id"`
	Labels []string       `json:"labels"`
	Props  map[string]any `json:"props"`
}

// Rel is the row shape returned by relationship reads.
type Rel struct {
	ID           int64          `json:"id"`
	Type         string         `json:"type"`
	StartID      int64          `json:"start_id,omitempty"`
	TargetID     int64          `json:"target_id,omitempty"`
	SourceID     int64          `json:"source_id,omitempty"`
	EndID        int64          `json:"end_id,omitempty"`
	StartLabels  []string       `json:"start_labels,omitempty"`
	TargetLabels []string       `json:"target_labels,omitempty"`
	SourceLabels []string       `json:"source_labels,omitempty"`
	EndLabels    []string       `json:"end_labels,omitempty"`
	Props        map[string]any `json:"props"`
}

// EntityDetail is the /entities/:id and /entities list element shape.
type EntityDetail struct {
	ID      int64          `json:"id"`
	Labels  []string       `json:"labels"`
	Props   map[string]any `json:"props"`
	OutRels []RelEdge      `json:"out_rels"`
	InRels  []RelEdge      `json:"in_rels"`
}

// RelEdge is an edge in the EntityDetail adjacency lists.
type RelEdge struct {
	ID           int64          `json:"id"`
	Type         string         `json:"type"`
	TargetID     int64          `json:"target_id,omitempty"`
	TargetLabels []string       `json:"target_labels,omitempty"`
	SourceID     int64          `json:"source_id,omitempty"`
	SourceLabels []string       `json:"source_labels,omitempty"`
	Props        map[string]any `json:"props"`
}

// ErrNotFound is returned by GetEntity when the id has no node.
var ErrNotFound = errors.New("entity not found")

// CreateNode creates a node with the given label and props, returns its id + persisted props.
func (s *Store) CreateNode(ctx context.Context, label string, props map[string]any) (*EntityDetail, error) {
	if !isIdentifier(label) {
		return nil, fmt.Errorf("invalid label %q", label)
	}
	cypher := fmt.Sprintf("CREATE (n:`%s`) SET n = $props RETURN id(n) AS id, properties(n) AS props", label)
	rec, err := s.runSingle(ctx, cypher, map[string]any{"props": props})
	if err != nil {
		return nil, err
	}
	id, _ := rec["id"].(int64)
	p, _ := rec["props"].(map[string]any)
	return &EntityDetail{
		ID:      id,
		Labels:  []string{label},
		Props:   p,
		OutRels: []RelEdge{},
		InRels:  []RelEdge{},
	}, nil
}

// UpdateNode applies a sparse patch (SET for set, REMOVE for unset).
func (s *Store) UpdateNode(ctx context.Context, id int64, set map[string]any, unset []string) error {
	params := map[string]any{"id": id}
	body := ""
	i := 0
	for k := range set {
		if !isIdentifier(k) {
			return fmt.Errorf("invalid prop key %q", k)
		}
	}
	for _, k := range unset {
		if !isIdentifier(k) {
			return fmt.Errorf("invalid prop key %q", k)
		}
	}
	for k, v := range set {
		pkey := fmt.Sprintf("p_%d", i)
		body += fmt.Sprintf(" SET n.`%s` = $%s", k, pkey)
		params[pkey] = v
		i++
	}
	for _, k := range unset {
		body += fmt.Sprintf(" REMOVE n.`%s`", k)
	}
	cypher := fmt.Sprintf("MATCH (n) WHERE id(n) = $id%s RETURN id(n) AS id", body)
	_, err := s.runSingle(ctx, cypher, params)
	return err
}

// DeleteNode removes a node, optionally detaching its relationships first.
func (s *Store) DeleteNode(ctx context.Context, id int64, cascade bool) error {
	var cypher string
	if cascade {
		cypher = "MATCH (n) WHERE id(n) = $id DETACH DELETE n"
	} else {
		cypher = "MATCH (n) WHERE id(n) = $id DELETE n"
	}
	return s.runVoid(ctx, cypher, map[string]any{"id": id})
}

// GetEntity returns the full detail view of a node.
func (s *Store) GetEntity(ctx context.Context, id int64) (*EntityDetail, error) {
	cypher := `MATCH (n) WHERE id(n) = $id
RETURN id(n) AS id,
       labels(n) AS labels,
       properties(n) AS props,
       [(n)-[r]->(t) | {id: id(r), type: type(r), target_id: id(t), target_labels: labels(t), props: properties(r)}] AS out_rels,
       [(src)-[r]->(n) | {id: id(r), type: type(r), source_id: id(src), source_labels: labels(src), props: properties(r)}] AS in_rels`
	rec, err := s.runSingle(ctx, cypher, map[string]any{"id": id})
	if err != nil {
		return nil, err
	}
	if rec == nil {
		return nil, ErrNotFound
	}
	d := &EntityDetail{
		ID:     mustInt64(rec["id"]),
		Labels: mustStringSlice(rec["labels"]),
		Props:  asPropMap(rec["props"]),
	}
	d.OutRels = decodeRelEdges(rec["out_rels"], true)
	d.InRels = decodeRelEdges(rec["in_rels"], false)
	return d, nil
}

// ListEntities filters by label and/or substring query.
func (s *Store) ListEntities(ctx context.Context, label, q string, limit int) ([]EntityDetail, error) {
	var cypher string
	params := map[string]any{"lim": int64(limit)}
	switch {
	case label != "" && q != "":
		if !isIdentifier(label) {
			return nil, fmt.Errorf("invalid label %q", label)
		}
		cypher = fmt.Sprintf("MATCH (n:`%s`) WHERE any(k IN keys(n) WHERE toLower(toString(n[k])) CONTAINS toLower($q)) RETURN id(n) AS id, labels(n) AS labels, properties(n) AS props LIMIT $lim", label)
		params["q"] = q
	case label != "":
		if !isIdentifier(label) {
			return nil, fmt.Errorf("invalid label %q", label)
		}
		cypher = fmt.Sprintf("MATCH (n:`%s`) RETURN id(n) AS id, labels(n) AS labels, properties(n) AS props LIMIT $lim", label)
	case q != "":
		cypher = "MATCH (n) WHERE any(k IN keys(n) WHERE toLower(toString(n[k])) CONTAINS toLower($q)) RETURN id(n) AS id, labels(n) AS labels, properties(n) AS props LIMIT $lim"
		params["q"] = q
	default:
		cypher = "MATCH (n) RETURN id(n) AS id, labels(n) AS labels, properties(n) AS props LIMIT $lim"
	}
	records, err := s.runMany(ctx, cypher, params)
	if err != nil {
		return nil, err
	}
	out := make([]EntityDetail, 0, len(records))
	for _, r := range records {
		out = append(out, EntityDetail{
			ID:      mustInt64(r["id"]),
			Labels:  mustStringSlice(r["labels"]),
			Props:   asPropMap(r["props"]),
			OutRels: []RelEdge{},
			InRels:  []RelEdge{},
		})
	}
	return out, nil
}

// CreateLink builds a typed relationship between two existing nodes.
func (s *Store) CreateLink(ctx context.Context, ty string, startID, endID int64, props map[string]any) (int64, error) {
	if !isIdentifier(ty) {
		return 0, fmt.Errorf("invalid rel type %q", ty)
	}
	cypher := fmt.Sprintf("MATCH (src) WHERE id(src) = $sid MATCH (dst) WHERE id(dst) = $eid CREATE (src)-[r:`%s` $p]->(dst) RETURN id(r) AS id", ty)
	rec, err := s.runSingle(ctx, cypher, map[string]any{"sid": startID, "eid": endID, "p": props})
	if err != nil {
		return 0, err
	}
	return mustInt64(rec["id"]), nil
}

// DeleteLink removes a relationship by id.
func (s *Store) DeleteLink(ctx context.Context, id int64) error {
	return s.runVoid(ctx, "MATCH ()-[r]->() WHERE id(r) = $id DELETE r", map[string]any{"id": id})
}

// LookupLabels returns the first label of two nodes; used for endpoint validation.
func (s *Store) LookupLabels(ctx context.Context, startID, endID int64) (string, string, error) {
	rec, err := s.runSingle(ctx, "MATCH (s) WHERE id(s) = $sid MATCH (e) WHERE id(e) = $eid RETURN labels(s) AS sl, labels(e) AS el", map[string]any{"sid": startID, "eid": endID})
	if err != nil {
		return "", "", err
	}
	if rec == nil {
		return "", "", ErrNotFound
	}
	sl := firstString(rec["sl"])
	el := firstString(rec["el"])
	return sl, el, nil
}

// --- helpers --------------------------------------------------------------

func (s *Store) runSingle(ctx context.Context, cypher string, params map[string]any) (map[string]any, error) {
	res, err := neo4j.ExecuteQuery[*neo4j.EagerResult](ctx, s.driver, cypher, params, neo4j.EagerResultTransformer)
	if err != nil {
		return nil, err
	}
	if len(res.Records) == 0 {
		return nil, nil
	}
	return res.Records[0].AsMap(), nil
}

func (s *Store) runMany(ctx context.Context, cypher string, params map[string]any) ([]map[string]any, error) {
	res, err := neo4j.ExecuteQuery[*neo4j.EagerResult](ctx, s.driver, cypher, params, neo4j.EagerResultTransformer)
	if err != nil {
		return nil, err
	}
	out := make([]map[string]any, 0, len(res.Records))
	for _, r := range res.Records {
		out = append(out, r.AsMap())
	}
	return out, nil
}

func (s *Store) runVoid(ctx context.Context, cypher string, params map[string]any) error {
	_, err := neo4j.ExecuteQuery[*neo4j.EagerResult](ctx, s.driver, cypher, params, neo4j.EagerResultTransformer)
	return err
}

func isIdentifier(s string) bool {
	if s == "" {
		return false
	}
	for i, r := range s {
		ok := r == '_' || (r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z')
		if i > 0 {
			ok = ok || (r >= '0' && r <= '9')
		}
		if !ok {
			return false
		}
	}
	return true
}

func mustInt64(v any) int64 {
	switch n := v.(type) {
	case int64:
		return n
	case int:
		return int64(n)
	case float64:
		return int64(n)
	}
	return 0
}

func mustStringSlice(v any) []string {
	if v == nil {
		return nil
	}
	switch xs := v.(type) {
	case []any:
		out := make([]string, 0, len(xs))
		for _, x := range xs {
			if s, ok := x.(string); ok {
				out = append(out, s)
			}
		}
		return out
	case []string:
		return xs
	}
	return nil
}

func asPropMap(v any) map[string]any {
	if v == nil {
		return map[string]any{}
	}
	if m, ok := v.(map[string]any); ok {
		return m
	}
	return map[string]any{}
}

func firstString(v any) string {
	xs := mustStringSlice(v)
	if len(xs) == 0 {
		return ""
	}
	return xs[0]
}

func decodeRelEdges(v any, outgoing bool) []RelEdge {
	out := []RelEdge{}
	xs, ok := v.([]any)
	if !ok {
		return out
	}
	for _, x := range xs {
		m, ok := x.(map[string]any)
		if !ok {
			continue
		}
		// OPTIONAL MATCH null guard
		if _, has := m["id"]; !has || m["id"] == nil {
			continue
		}
		id := mustInt64(m["id"])
		if id == 0 {
			if m["id"] == nil {
				continue
			}
		}
		e := RelEdge{
			ID:    mustInt64(m["id"]),
			Type:  fmt.Sprint(m["type"]),
			Props: asPropMap(m["props"]),
		}
		if outgoing {
			e.TargetID = mustInt64(m["target_id"])
			e.TargetLabels = mustStringSlice(m["target_labels"])
		} else {
			e.SourceID = mustInt64(m["source_id"])
			e.SourceLabels = mustStringSlice(m["source_labels"])
		}
		out = append(out, e)
	}
	return out
}
