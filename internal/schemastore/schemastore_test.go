//go:build integration

package schemastore

import (
	"context"
	"os"
	"testing"

	"github.com/neo4j/neo4j-go-driver/v5/neo4j"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	"github.com/plutonium-guy/kg_editor/internal/schema"
)

func newDriver(t *testing.T) neo4j.DriverWithContext {
	t.Helper()
	uri := os.Getenv("NEO4J_URI")
	pw := os.Getenv("NEO4J_PASSWORD")
	if uri == "" || pw == "" {
		t.Skip("NEO4J_URI / NEO4J_PASSWORD not set")
	}
	d, err := neo4j.NewDriverWithContext(uri, neo4j.BasicAuth("neo4j", pw, ""))
	require.NoError(t, err)
	require.NoError(t, d.VerifyConnectivity(context.Background()))
	return d
}

func sampleSchema() *schema.File {
	return &schema.File{
		Nodes: map[string]schema.NodeDef{
			"Person": {
				Description: "Test person",
				Props: []schema.FieldSpec{
					// Unique intentionally omitted: the live DB has duplicate Person.name values;
					// materializing a uniqueness constraint against those would fail.
					// TestUniqueConstraintMaterialized covers unique-constraint materialization
					// using the isolated label TestUniq.
					{Name: "name", FieldType: schema.FieldType{Type: "string"}, Required: true},
					{Name: "role", FieldType: schema.FieldType{Type: "enum", Values: []string{"a", "b"}}},
				},
				Indexes: [][]string{{"name"}},
			},
			"Company": {
				Description: "Test company",
				Props:       []schema.FieldSpec{{Name: "name", FieldType: schema.FieldType{Type: "string"}, Required: true}},
			},
		},
		Rels: map[string]schema.RelDef{
			"WORKS_AT": {
				Endpoints:   [][2]string{{"Person", "Company"}},
				Cardinality: "many_to_many",
				Props:       []schema.FieldSpec{{Name: "role", FieldType: schema.FieldType{Type: "string"}}},
			},
		},
	}
}

func TestBootstrapEmpty(t *testing.T) {
	ctx := context.Background()
	d := newDriver(t)
	defer d.Close(ctx)
	store := New(d)

	// wipe any existing schema
	require.NoError(t, store.Save(ctx, &schema.File{Nodes: map[string]schema.NodeDef{}, Rels: map[string]schema.RelDef{}}))

	got, err := store.Bootstrap(ctx, sampleSchema())
	require.NoError(t, err)
	assert.Contains(t, got.Nodes, "Person")
	assert.Contains(t, got.Rels, "WORKS_AT")

	// second bootstrap should return same data without re-seeding
	got2, err := store.Bootstrap(ctx, &schema.File{})
	require.NoError(t, err)
	assert.Equal(t, got.Nodes, got2.Nodes)
}

func TestSaveLoadRoundtrip(t *testing.T) {
	ctx := context.Background()
	d := newDriver(t)
	defer d.Close(ctx)
	store := New(d)
	in := sampleSchema()

	require.NoError(t, store.Save(ctx, in))
	out, err := store.Load(ctx)
	require.NoError(t, err)

	assert.Equal(t, in.Nodes["Person"].Description, out.Nodes["Person"].Description)
	assert.Equal(t, in.Nodes["Person"].Indexes, out.Nodes["Person"].Indexes)
	assert.Len(t, out.Nodes["Person"].Props, 2)
	assert.Equal(t, "name", out.Nodes["Person"].Props[0].Name)
	assert.Equal(t, "enum", out.Nodes["Person"].Props[1].Type)
	assert.Equal(t, []string{"a", "b"}, out.Nodes["Person"].Props[1].Values)
	assert.Len(t, out.Rels["WORKS_AT"].Endpoints, 1)
	assert.Equal(t, [2]string{"Person", "Company"}, out.Rels["WORKS_AT"].Endpoints[0])
}

func TestSaveNodeDeleteNode(t *testing.T) {
	ctx := context.Background()
	d := newDriver(t)
	defer d.Close(ctx)
	store := New(d)
	require.NoError(t, store.Save(ctx, sampleSchema()))

	// Add a new label
	require.NoError(t, store.SaveNode(ctx, "Skill", schema.NodeDef{
		Description: "A skill",
		Props:       []schema.FieldSpec{{Name: "name", FieldType: schema.FieldType{Type: "string"}, Required: true}},
	}))
	f, err := store.Load(ctx)
	require.NoError(t, err)
	assert.Contains(t, f.Nodes, "Skill")

	require.NoError(t, store.DeleteNode(ctx, "Skill"))
	f, err = store.Load(ctx)
	require.NoError(t, err)
	assert.NotContains(t, f.Nodes, "Skill")
}

func TestUniqueConstraintMaterialized(t *testing.T) {
	ctx := context.Background()
	d := newDriver(t)
	defer d.Close(ctx)
	store := New(d)

	// Clean slate
	require.NoError(t, store.Save(ctx, &schema.File{Nodes: map[string]schema.NodeDef{}, Rels: map[string]schema.RelDef{}}))
	require.NoError(t, store.DropNodeConstraints(ctx, "TestUniq"))

	def := schema.NodeDef{
		Description: "x",
		Props: []schema.FieldSpec{
			{Name: "code", FieldType: schema.FieldType{Type: "string"}, Required: true, Unique: true},
		},
	}
	require.NoError(t, store.SaveNode(ctx, "TestUniq", def))

	// Verify constraint exists via SHOW CONSTRAINTS.
	res, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
		ctx, d,
		"SHOW CONSTRAINTS YIELD name WHERE name STARTS WITH 'kg_TestUniq_' RETURN count(*) AS n",
		nil, neo4j.EagerResultTransformer,
	)
	require.NoError(t, err)
	rec := res.Records[0].AsMap()
	cnt, _ := rec["n"].(int64)
	assert.GreaterOrEqual(t, cnt, int64(1), "expected at least one constraint named kg_TestUniq_*")

	// Delete label drops constraint
	require.NoError(t, store.DeleteNode(ctx, "TestUniq"))
	res2, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
		ctx, d,
		"SHOW CONSTRAINTS YIELD name WHERE name STARTS WITH 'kg_TestUniq_' RETURN count(*) AS n",
		nil, neo4j.EagerResultTransformer,
	)
	require.NoError(t, err)
	cnt2, _ := res2.Records[0].AsMap()["n"].(int64)
	assert.Equal(t, int64(0), cnt2, "constraint should be dropped after label delete")
}

func TestIndexMaterialized(t *testing.T) {
	ctx := context.Background()
	d := newDriver(t)
	defer d.Close(ctx)
	store := New(d)

	require.NoError(t, store.DropNodeConstraints(ctx, "TestIdx"))
	def := schema.NodeDef{
		Props:   []schema.FieldSpec{{Name: "name", FieldType: schema.FieldType{Type: "string"}}},
		Indexes: [][]string{{"name"}},
	}
	require.NoError(t, store.SaveNode(ctx, "TestIdx", def))

	res, err := neo4j.ExecuteQuery[*neo4j.EagerResult](
		ctx, d,
		"SHOW INDEXES YIELD name WHERE name STARTS WITH 'kg_TestIdx_' RETURN count(*) AS n",
		nil, neo4j.EagerResultTransformer,
	)
	require.NoError(t, err)
	cnt, _ := res.Records[0].AsMap()["n"].(int64)
	assert.GreaterOrEqual(t, cnt, int64(1), "expected at least one index")

	require.NoError(t, store.DeleteNode(ctx, "TestIdx"))
}
