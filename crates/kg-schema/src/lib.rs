//! YAML-driven schema loader.

pub mod error;
pub mod field;

pub use error::SchemaError;
pub use field::{FieldSpec, FieldType};

use kg_core::schema::{Cardinality, NodeSchema, PropType, RelSchema, SchemaRegistry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchemaFile {
    #[serde(default)]
    pub nodes: HashMap<String, NodeDef>,
    #[serde(default)]
    pub rels:  HashMap<String, RelDef>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeDef {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub props: Vec<FieldSpec>,
    #[serde(default)]
    pub indexes: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelDef {
    #[serde(default)]
    pub endpoints: Vec<(String, String)>,
    #[serde(default = "default_cardinality")]
    pub cardinality: String,
    #[serde(default)]
    pub props: Vec<FieldSpec>,
}

fn default_cardinality() -> String { "many_to_many".into() }

impl SchemaFile {
    pub fn from_yaml(path: &Path) -> Result<Self, SchemaError> {
        let raw = std::fs::read_to_string(path)?;
        let parsed: SchemaFile = serde_yaml::from_str(&raw)?;
        parsed.validate()?;
        Ok(parsed)
    }

    pub fn validate(&self) -> Result<(), SchemaError> {
        let labels: std::collections::HashSet<&str> = self.nodes.keys().map(String::as_str).collect();
        for (label, def) in &self.nodes {
            for spec in &def.props {
                if let FieldType::Ref { label: target } = &spec.ty {
                    if !labels.contains(target.as_str()) {
                        return Err(SchemaError::Invalid(format!(
                            "node `{label}` field `{}` references unknown label `{target}`",
                            spec.name
                        )));
                    }
                }
            }
        }
        for (ty, def) in &self.rels {
            for (s, e) in &def.endpoints {
                if !labels.contains(s.as_str()) {
                    return Err(SchemaError::Invalid(format!("rel `{ty}` endpoint start `{s}` not a declared node")));
                }
                if !labels.contains(e.as_str()) {
                    return Err(SchemaError::Invalid(format!("rel `{ty}` endpoint end `{e}` not a declared node")));
                }
            }
        }
        Ok(())
    }

    pub fn to_core_registry(&self) -> SchemaRegistry {
        let mut reg = SchemaRegistry::new();
        for (label, def) in &self.nodes {
            let mut b = NodeSchema::builder(label);
            for spec in &def.props {
                let pt = match &spec.ty {
                    FieldType::String | FieldType::Date | FieldType::DateTime | FieldType::Enum { .. } => PropType::String,
                    FieldType::Int    | FieldType::Ref { .. } => PropType::Int,
                    FieldType::Float  => PropType::Float,
                    FieldType::Bool   => PropType::Bool,
                };
                b = b.prop(&spec.name, pt);
                if spec.required { b = b.required(); }
            }
            for ix in &def.indexes { b = b.index(ix.iter().cloned()); }
            reg.add_node(b.build());
        }
        for (ty, def) in &self.rels {
            let mut b = RelSchema::builder(ty);
            for (s, e) in &def.endpoints { b = b.endpoints(s, e); }
            let card = match def.cardinality.as_str() {
                "one_to_one"  => Cardinality::OneToOne,
                "one_to_many" => Cardinality::OneToMany,
                _             => Cardinality::ManyToMany,
            };
            b = b.cardinality(card);
            reg.add_rel(b.build());
        }
        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_yaml(s: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(s.as_bytes()).unwrap();
        f
    }

    #[test]
    fn parses_example_schema() {
        let f = write_yaml(include_str!("../../../kg-schema.yaml"));
        let s = SchemaFile::from_yaml(f.path()).unwrap();
        assert!(s.nodes.contains_key("Person"));
        assert!(s.rels.contains_key("KNOWS"));
    }

    #[test]
    fn rejects_ref_to_unknown_label() {
        let f = write_yaml(r#"
nodes:
  Foo:
    props:
      - { name: bar, type: ref, label: NotALabel }
"#);
        let e = SchemaFile::from_yaml(f.path()).unwrap_err();
        assert!(matches!(e, SchemaError::Invalid(_)));
    }

    #[test]
    fn rejects_rel_endpoint_to_unknown_label() {
        let f = write_yaml(r#"
nodes:
  Foo:
    props: []
rels:
  KNOWS:
    endpoints: [[Foo, Bar]]
"#);
        assert!(matches!(SchemaFile::from_yaml(f.path()).unwrap_err(), SchemaError::Invalid(_)));
    }

    #[test]
    fn projects_to_core_registry() {
        let f = write_yaml(include_str!("../../../kg-schema.yaml"));
        let s = SchemaFile::from_yaml(f.path()).unwrap();
        let reg = s.to_core_registry();
        assert!(reg.node("Person").is_some());
        assert!(reg.rel("KNOWS").is_some());
    }
}
