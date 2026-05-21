// Package schemastore reads + writes the schema as nodes/edges in Neo4j.
// On first server boot, falls back to a YAML seed if Neo4j has no _Schema nodes.
package schemastore

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/neo4j/neo4j-go-driver/v5/neo4j"

	"github.com/plutonium-guy/kg_editor/internal/schema"
)

type Store struct {
	driver neo4j.DriverWithContext
}

func New(driver neo4j.DriverWithContext) *Store { return &Store{driver: driver} }

// Bootstrap loads schema from Neo4j; if none exists, writes the supplied seed
// and returns it.
func (s *Store) Bootstrap(ctx context.Context, seed *schema.File) (*schema.File, error) {
	loaded, err := s.Load(ctx)
	if err != nil {
		return nil, err
	}
	if len(loaded.Nodes) == 0 && len(loaded.Rels) == 0 {
		if seed == nil {
			return loaded, nil
		}
		if err := s.Save(ctx, seed); err != nil {
			return nil, fmt.Errorf("seed schema: %w", err)
		}
		return s.Load(ctx)
	}
	return loaded, nil
}

// Load reads the full SchemaFile from Neo4j.
func (s *Store) Load(ctx context.Context) (*schema.File, error) {
	out := &schema.File{
		Nodes: map[string]schema.NodeDef{},
		Rels:  map[string]schema.RelDef{},
	}

	// Nodes
	nodeRes, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
		ctx, s.driver,
		`MATCH (n:_Schema:_NodeLabel)
		 OPTIONAL MATCH (n)-[hp:HAS_PROP]->(p:_PropSpec)
		 OPTIONAL MATCH (n)-[hi:HAS_INDEX]->(i:_Index)
		 RETURN n.name AS name, n.description AS description,
		        collect(DISTINCT {position: hp.position, prop: properties(p)}) AS props,
		        collect(DISTINCT {position: hi.position, idx: properties(i)})  AS idxs`,
		nil, neo4j.EagerResultTransformer,
	)
	if err != nil {
		return nil, err
	}
	for _, rec := range nodeRes.Records {
		m := rec.AsMap()
		name, _ := m["name"].(string)
		if name == "" {
			continue
		}
		def := schema.NodeDef{
			Description: toString(m["description"]),
			Props:       decodeFields(m["props"]),
			Indexes:     decodeIndexes(m["idxs"]),
		}
		out.Nodes[name] = def
	}

	// Rels
	relRes, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
		ctx, s.driver,
		`MATCH (r:_Schema:_RelType)
		 OPTIONAL MATCH (r)-[hp:HAS_PROP]->(p:_PropSpec)
		 OPTIONAL MATCH (r)-[af:ALLOWED_FROM]->(sn:_NodeLabel)
		 OPTIONAL MATCH (r)-[at:ALLOWED_TO]->(en:_NodeLabel)
		 RETURN r.type AS type, r.cardinality AS cardinality,
		        collect(DISTINCT {position: hp.position, prop: properties(p)}) AS props,
		        collect(DISTINCT {position: af.position, name: sn.name}) AS allowed_from,
		        collect(DISTINCT {position: at.position, name: en.name}) AS allowed_to`,
		nil, neo4j.EagerResultTransformer,
	)
	if err != nil {
		return nil, err
	}
	for _, rec := range relRes.Records {
		m := rec.AsMap()
		ty, _ := m["type"].(string)
		if ty == "" {
			continue
		}
		card := toString(m["cardinality"])
		if card == "" {
			card = "many_to_many"
		}
		fromList := decodeOrderedNames(m["allowed_from"])
		toList := decodeOrderedNames(m["allowed_to"])
		// Pair by index. Endpoints in schema.RelDef are [start,end] pairs.
		eps := [][2]string{}
		n := len(fromList)
		if len(toList) < n {
			n = len(toList)
		}
		for i := 0; i < n; i++ {
			eps = append(eps, [2]string{fromList[i], toList[i]})
		}
		out.Rels[ty] = schema.RelDef{
			Endpoints:   eps,
			Cardinality: card,
			Props:       decodeFields(m["props"]),
		}
	}
	return out, nil
}

// Save replaces the entire stored schema atomically (within one tx).
func (s *Store) Save(ctx context.Context, f *schema.File) error {
	// Single big transaction: wipe + rebuild.
	_, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
		ctx, s.driver,
		"MATCH (n:_Schema) DETACH DELETE n",
		nil, neo4j.EagerResultTransformer,
	)
	if err != nil {
		return fmt.Errorf("clear schema: %w", err)
	}

	// Nodes
	for label, def := range f.Nodes {
		if err := s.upsertNode(ctx, label, def); err != nil {
			return err
		}
	}
	// Rels (after nodes so ALLOWED_FROM/TO can MATCH them)
	for ty, def := range f.Rels {
		if err := s.upsertRel(ctx, ty, def); err != nil {
			return err
		}
	}
	return nil
}

// SaveNode upserts a single node label.
func (s *Store) SaveNode(ctx context.Context, label string, def schema.NodeDef) error {
	// Delete existing label + props/indexes first to avoid stale data.
	_, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
		ctx, s.driver,
		`MATCH (n:_Schema:_NodeLabel {name:$name})
		 OPTIONAL MATCH (n)-[:HAS_PROP]->(p:_PropSpec)
		 OPTIONAL MATCH (n)-[:HAS_INDEX]->(i:_Index)
		 DETACH DELETE n, p, i`,
		map[string]any{"name": label}, neo4j.EagerResultTransformer,
	)
	if err != nil {
		return err
	}
	return s.upsertNode(ctx, label, def)
}

// DeleteNode removes a label.
func (s *Store) DeleteNode(ctx context.Context, label string) error {
	_, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
		ctx, s.driver,
		`MATCH (n:_Schema:_NodeLabel {name:$name})
		 OPTIONAL MATCH (n)-[:HAS_PROP]->(p:_PropSpec)
		 OPTIONAL MATCH (n)-[:HAS_INDEX]->(i:_Index)
		 DETACH DELETE n, p, i`,
		map[string]any{"name": label}, neo4j.EagerResultTransformer,
	)
	return err
}

// SaveRel upserts a single rel type.
func (s *Store) SaveRel(ctx context.Context, ty string, def schema.RelDef) error {
	_, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
		ctx, s.driver,
		`MATCH (r:_Schema:_RelType {type:$type})
		 OPTIONAL MATCH (r)-[:HAS_PROP]->(p:_PropSpec)
		 DETACH DELETE r, p`,
		map[string]any{"type": ty}, neo4j.EagerResultTransformer,
	)
	if err != nil {
		return err
	}
	return s.upsertRel(ctx, ty, def)
}

// DeleteRel removes a rel type.
func (s *Store) DeleteRel(ctx context.Context, ty string) error {
	_, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
		ctx, s.driver,
		`MATCH (r:_Schema:_RelType {type:$type})
		 OPTIONAL MATCH (r)-[:HAS_PROP]->(p:_PropSpec)
		 DETACH DELETE r, p`,
		map[string]any{"type": ty}, neo4j.EagerResultTransformer,
	)
	return err
}

// --- helpers ---

func (s *Store) upsertNode(ctx context.Context, label string, def schema.NodeDef) error {
	_, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
		ctx, s.driver,
		"CREATE (n:_Schema:_NodeLabel {name:$name, description:$desc})",
		map[string]any{"name": label, "desc": def.Description},
		neo4j.EagerResultTransformer,
	)
	if err != nil {
		return err
	}
	for i, p := range def.Props {
		if err := s.attachProp(ctx, "n:_Schema:_NodeLabel {name:$owner}", label, "n", i, p); err != nil {
			return err
		}
	}
	for i, ix := range def.Indexes {
		pj, _ := json.Marshal(ix)
		_, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
			ctx, s.driver,
			`MATCH (n:_Schema:_NodeLabel {name:$owner})
			 CREATE (n)-[:HAS_INDEX {position:$pos}]->(:_Index {props_json:$pj})`,
			map[string]any{"owner": label, "pos": int64(i), "pj": string(pj)},
			neo4j.EagerResultTransformer,
		)
		if err != nil {
			return err
		}
	}
	return nil
}

func (s *Store) upsertRel(ctx context.Context, ty string, def schema.RelDef) error {
	_, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
		ctx, s.driver,
		"CREATE (r:_Schema:_RelType {type:$ty, cardinality:$card})",
		map[string]any{"ty": ty, "card": def.Cardinality},
		neo4j.EagerResultTransformer,
	)
	if err != nil {
		return err
	}
	for i, ep := range def.Endpoints {
		// ALLOWED_FROM
		_, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
			ctx, s.driver,
			`MATCH (r:_Schema:_RelType {type:$ty})
			 MATCH (n:_Schema:_NodeLabel {name:$from})
			 CREATE (r)-[:ALLOWED_FROM {position:$pos}]->(n)`,
			map[string]any{"ty": ty, "from": ep[0], "pos": int64(i)},
			neo4j.EagerResultTransformer,
		)
		if err != nil {
			return err
		}
		// ALLOWED_TO
		_, err = neo4j.ExecuteQuery[*neo4j.EagerResult](
			ctx, s.driver,
			`MATCH (r:_Schema:_RelType {type:$ty})
			 MATCH (n:_Schema:_NodeLabel {name:$to})
			 CREATE (r)-[:ALLOWED_TO {position:$pos}]->(n)`,
			map[string]any{"ty": ty, "to": ep[1], "pos": int64(i)},
			neo4j.EagerResultTransformer,
		)
		if err != nil {
			return err
		}
	}
	for i, p := range def.Props {
		if err := s.attachProp(ctx, "r:_Schema:_RelType {type:$owner}", ty, "r", i, p); err != nil {
			return err
		}
	}
	return nil
}

func (s *Store) attachProp(ctx context.Context, ownerPattern, ownerKey, ownerVar string, pos int, p schema.FieldSpec) error {
	valuesJSON := ""
	if len(p.Values) > 0 {
		b, _ := json.Marshal(p.Values)
		valuesJSON = string(b)
	}
	defaultJSON := ""
	if p.Default != nil {
		b, _ := json.Marshal(p.Default)
		defaultJSON = string(b)
	}
	cypher := fmt.Sprintf(
		`MATCH (%s)
		 CREATE (%s)-[:HAS_PROP {position:$pos}]->(:_PropSpec {
		   name:$name, type:$type, values_json:$values, ref_label:$ref,
		   required:$req, unique:$uniq, default_json:$def
		 })`,
		ownerPattern, ownerVar,
	)
	_, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
		ctx, s.driver, cypher,
		map[string]any{
			"owner": ownerKey, "pos": int64(pos),
			"name": p.Name, "type": p.Type,
			"values": valuesJSON, "ref": p.Label,
			"req": p.Required, "uniq": p.Unique,
			"def": defaultJSON,
		},
		neo4j.EagerResultTransformer,
	)
	return err
}

func toString(v any) string {
	if s, ok := v.(string); ok {
		return s
	}
	return ""
}

func decodeFields(v any) []schema.FieldSpec {
	xs, ok := v.([]any)
	if !ok {
		return nil
	}
	type entry struct {
		pos  int64
		spec schema.FieldSpec
	}
	tmp := make([]entry, 0, len(xs))
	for _, x := range xs {
		m, ok := x.(map[string]any)
		if !ok {
			continue
		}
		prop, ok := m["prop"].(map[string]any)
		if !ok {
			continue
		}
		name, _ := prop["name"].(string)
		if name == "" {
			continue
		}
		spec := schema.FieldSpec{Name: name}
		spec.Type = toString(prop["type"])
		spec.Label = toString(prop["ref_label"])
		spec.Required, _ = prop["required"].(bool)
		spec.Unique, _ = prop["unique"].(bool)
		if vj := toString(prop["values_json"]); vj != "" {
			_ = json.Unmarshal([]byte(vj), &spec.Values)
		}
		if dj := toString(prop["default_json"]); dj != "" {
			var v any
			if err := json.Unmarshal([]byte(dj), &v); err == nil {
				spec.Default = v
			}
		}
		var pos int64
		switch p := m["position"].(type) {
		case int64:
			pos = p
		case float64:
			pos = int64(p)
		}
		tmp = append(tmp, entry{pos: pos, spec: spec})
	}
	// sort by position
	for i := 1; i < len(tmp); i++ {
		for j := i; j > 0 && tmp[j-1].pos > tmp[j].pos; j-- {
			tmp[j-1], tmp[j] = tmp[j], tmp[j-1]
		}
	}
	out := make([]schema.FieldSpec, 0, len(tmp))
	for _, e := range tmp {
		out = append(out, e.spec)
	}
	return out
}

func decodeIndexes(v any) [][]string {
	xs, ok := v.([]any)
	if !ok {
		return nil
	}
	type entry struct {
		pos  int64
		cols []string
	}
	tmp := make([]entry, 0, len(xs))
	for _, x := range xs {
		m, ok := x.(map[string]any)
		if !ok {
			continue
		}
		idx, ok := m["idx"].(map[string]any)
		if !ok {
			continue
		}
		pj, _ := idx["props_json"].(string)
		if pj == "" {
			continue
		}
		var cols []string
		if err := json.Unmarshal([]byte(pj), &cols); err != nil {
			continue
		}
		var pos int64
		switch p := m["position"].(type) {
		case int64:
			pos = p
		case float64:
			pos = int64(p)
		}
		tmp = append(tmp, entry{pos: pos, cols: cols})
	}
	for i := 1; i < len(tmp); i++ {
		for j := i; j > 0 && tmp[j-1].pos > tmp[j].pos; j-- {
			tmp[j-1], tmp[j] = tmp[j], tmp[j-1]
		}
	}
	out := make([][]string, 0, len(tmp))
	for _, e := range tmp {
		out = append(out, e.cols)
	}
	return out
}

func decodeOrderedNames(v any) []string {
	xs, ok := v.([]any)
	if !ok {
		return nil
	}
	type entry struct {
		pos  int64
		name string
	}
	tmp := make([]entry, 0, len(xs))
	for _, x := range xs {
		m, ok := x.(map[string]any)
		if !ok {
			continue
		}
		name, _ := m["name"].(string)
		if name == "" {
			continue
		}
		var pos int64
		switch p := m["position"].(type) {
		case int64:
			pos = p
		case float64:
			pos = int64(p)
		}
		tmp = append(tmp, entry{pos: pos, name: name})
	}
	for i := 1; i < len(tmp); i++ {
		for j := i; j > 0 && tmp[j-1].pos > tmp[j].pos; j-- {
			tmp[j-1], tmp[j] = tmp[j], tmp[j-1]
		}
	}
	out := make([]string, 0, len(tmp))
	for _, e := range tmp {
		out = append(out, e.name)
	}
	return out
}
