package schema

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func writeTempYAML(t *testing.T, body string) string {
	t.Helper()
	dir := t.TempDir()
	p := filepath.Join(dir, "schema.yaml")
	require.NoError(t, os.WriteFile(p, []byte(body), 0644))
	return p
}

func TestParsesExampleSchema(t *testing.T) {
	// Find the repo root kg-schema.yaml relative to this test file.
	body, err := os.ReadFile("../../kg-schema.yaml")
	require.NoError(t, err)
	p := writeTempYAML(t, string(body))
	f, err := FromYAML(p)
	require.NoError(t, err)
	assert.Contains(t, f.Nodes, "Person")
	assert.Contains(t, f.Rels, "KNOWS")
}

func TestRejectsRefToUnknownLabel(t *testing.T) {
	p := writeTempYAML(t, `
nodes:
  Foo:
    props:
      - { name: bar, type: ref, label: NotALabel }
`)
	_, err := FromYAML(p)
	require.Error(t, err)
	assert.Contains(t, err.Error(), "unknown label")
}

func TestRejectsRelEndpointToUnknownLabel(t *testing.T) {
	p := writeTempYAML(t, `
nodes:
  Foo:
    props: []
rels:
  KNOWS:
    endpoints: [[Foo, Bar]]
`)
	_, err := FromYAML(p)
	require.Error(t, err)
	assert.Contains(t, err.Error(), "endpoint")
}

func TestValidatePropsMissingRequired(t *testing.T) {
	specs := []FieldSpec{
		{Name: "name", FieldType: FieldType{Type: "string"}, Required: true},
		{Name: "age", FieldType: FieldType{Type: "int"}},
	}
	errs := ValidateProps(specs, map[string]any{"age": 30})
	assert.Len(t, errs, 1)
	assert.Contains(t, errs[0], "name")
}

func TestValidatePropsEnum(t *testing.T) {
	specs := []FieldSpec{
		{Name: "role", FieldType: FieldType{Type: "enum", Values: []string{"a", "b"}}},
	}
	assert.Empty(t, ValidateProps(specs, map[string]any{"role": "a"}))
	assert.NotEmpty(t, ValidateProps(specs, map[string]any{"role": "wizard"}))
}

func TestValidatePropsIntAcceptsJSONFloat(t *testing.T) {
	specs := []FieldSpec{{Name: "n", FieldType: FieldType{Type: "int"}}}
	// JSON unmarshals numbers as float64; ValidateProps must accept whole-number float64 as int.
	assert.Empty(t, ValidateProps(specs, map[string]any{"n": float64(42)}))
	assert.NotEmpty(t, ValidateProps(specs, map[string]any{"n": float64(1.5)}))
}

func TestEndpointsAllowed(t *testing.T) {
	r := RelDef{Endpoints: [][2]string{{"Person", "Person"}, {"Person", "Company"}}}
	assert.True(t, r.EndpointsAllowed("Person", "Person"))
	assert.True(t, r.EndpointsAllowed("Person", "Company"))
	assert.False(t, r.EndpointsAllowed("Person", "Skill"))
	r2 := RelDef{}
	assert.True(t, r2.EndpointsAllowed("Anything", "Anywhere"))
}
