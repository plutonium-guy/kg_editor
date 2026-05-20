// Package schema loads kg-schema.yaml and provides validation primitives
// used by the HTTP API and the MCP tools.
package schema

import (
	"fmt"
	"os"

	"gopkg.in/yaml.v3"
)

// FieldType carries the discriminated union shape used in YAML and JSON.
// `Type` is one of: string, int, float, bool, date, date_time, enum, ref.
// `Values` is populated for enum. `Label` is populated for ref.
type FieldType struct {
	Type   string   `yaml:"type"             json:"type"`
	Values []string `yaml:"values,omitempty" json:"values,omitempty"`
	Label  string   `yaml:"label,omitempty"  json:"label,omitempty"`
}

// FieldSpec describes a single property on a node or relationship.
type FieldSpec struct {
	Name      string `yaml:"name"             json:"name"`
	FieldType `yaml:",inline" json:",inline"`
	Required  bool `yaml:"required,omitempty" json:"required,omitempty"`
	Unique    bool `yaml:"unique,omitempty"   json:"unique,omitempty"`
	Default   any  `yaml:"default,omitempty"  json:"default,omitempty"`
}

// NodeDef describes a node label.
type NodeDef struct {
	Description string      `yaml:"description,omitempty" json:"description,omitempty"`
	Props       []FieldSpec `yaml:"props"                 json:"props"`
	Indexes     [][]string  `yaml:"indexes,omitempty"     json:"indexes,omitempty"`
}

// RelDef describes a relationship type.
type RelDef struct {
	Endpoints   [][2]string `yaml:"endpoints"             json:"endpoints"`
	Cardinality string      `yaml:"cardinality,omitempty" json:"cardinality"`
	Props       []FieldSpec `yaml:"props,omitempty"       json:"props"`
}

// File is the in-memory representation of kg-schema.yaml.
type File struct {
	Nodes map[string]NodeDef `yaml:"nodes" json:"nodes"`
	Rels  map[string]RelDef  `yaml:"rels"  json:"rels"`
}

// FromYAML parses + validates kg-schema.yaml from disk.
func FromYAML(path string) (*File, error) {
	raw, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read schema %q: %w", path, err)
	}
	var f File
	if err := yaml.Unmarshal(raw, &f); err != nil {
		return nil, fmt.Errorf("parse schema yaml: %w", err)
	}
	if err := f.Validate(); err != nil {
		return nil, err
	}
	return &f, nil
}

// Validate cross-references: ref fields point to declared labels, rel
// endpoints reference declared labels.
func (f *File) Validate() error {
	for label, def := range f.Nodes {
		for _, p := range def.Props {
			if p.Type == "ref" {
				if _, ok := f.Nodes[p.Label]; !ok {
					return fmt.Errorf("node %q field %q references unknown label %q", label, p.Name, p.Label)
				}
			}
			if p.Type == "enum" && len(p.Values) == 0 {
				return fmt.Errorf("node %q enum field %q has no values", label, p.Name)
			}
		}
	}
	for ty, def := range f.Rels {
		for _, ep := range def.Endpoints {
			if _, ok := f.Nodes[ep[0]]; !ok {
				return fmt.Errorf("rel %q endpoint start %q not a declared node", ty, ep[0])
			}
			if _, ok := f.Nodes[ep[1]]; !ok {
				return fmt.Errorf("rel %q endpoint end %q not a declared node", ty, ep[1])
			}
		}
	}
	return nil
}

// ValidateProps checks a props map against a slice of FieldSpec.
// Returns a slice of human-readable violations.
func ValidateProps(specs []FieldSpec, props map[string]any) []string {
	var errs []string
	for _, spec := range specs {
		v, present := props[spec.Name]
		if !present {
			if spec.Required {
				errs = append(errs, fmt.Sprintf("missing required field %q", spec.Name))
			}
			continue
		}
		if v == nil {
			continue
		}
		if !typeMatches(v, spec.FieldType) {
			errs = append(errs, fmt.Sprintf("field %q wrong type for spec %s", spec.Name, spec.Type))
		}
	}
	return errs
}

func typeMatches(v any, ft FieldType) bool {
	switch ft.Type {
	case "string", "date", "date_time":
		_, ok := v.(string)
		return ok
	case "int":
		switch n := v.(type) {
		case int, int32, int64:
			return true
		case float64:
			return n == float64(int64(n)) // JSON numbers come in as float64
		}
		return false
	case "float":
		switch v.(type) {
		case float32, float64, int, int32, int64:
			return true
		}
		return false
	case "bool":
		_, ok := v.(bool)
		return ok
	case "enum":
		s, ok := v.(string)
		if !ok {
			return false
		}
		for _, allowed := range ft.Values {
			if allowed == s {
				return true
			}
		}
		return false
	case "ref":
		switch v.(type) {
		case int, int32, int64:
			return true
		case float64:
			return true
		}
		return false
	}
	return true
}

// EndpointsAllowed returns true if (startLabel, endLabel) is in the rel's
// declared endpoints list. Empty list means no constraint.
func (d *RelDef) EndpointsAllowed(start, end string) bool {
	if len(d.Endpoints) == 0 {
		return true
	}
	for _, ep := range d.Endpoints {
		if ep[0] == start && ep[1] == end {
			return true
		}
	}
	return false
}
