//! Optional schema registry.

pub mod types;

pub use types::{Cardinality, NodeSchema, PropSpec, PropType, RelSchema};

use crate::error::SchemaViolation;
use crate::value::PropValue;
use std::collections::{BTreeMap, HashMap};

/// Holds declared node and relationship schemas.
#[derive(Clone, Debug, Default)]
pub struct SchemaRegistry {
    nodes: HashMap<String, NodeSchema>,
    rels: HashMap<String, RelSchema>,
}

impl SchemaRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn add_node(&mut self, s: NodeSchema) -> &mut Self {
        self.nodes.insert(s.label.clone(), s);
        self
    }
    pub fn add_rel(&mut self, s: RelSchema) -> &mut Self {
        self.rels.insert(s.r#type.clone(), s);
        self
    }

    pub fn node(&self, label: &str) -> Option<&NodeSchema> { self.nodes.get(label) }
    pub fn rel(&self, ty: &str) -> Option<&RelSchema> { self.rels.get(ty) }
    pub fn nodes(&self) -> impl Iterator<Item = &NodeSchema> { self.nodes.values() }
    pub fn rels(&self) -> impl Iterator<Item = &RelSchema> { self.rels.values() }

    /// Validate properties against the schema for `label`.
    /// Returns all collected violations. Unknown label is itself a violation.
    pub fn validate_node_props(
        &self,
        label: &str,
        props: &BTreeMap<String, PropValue>,
    ) -> Vec<SchemaViolation> {
        let Some(schema) = self.nodes.get(label) else {
            return vec![SchemaViolation::UnknownLabel { label: label.into() }];
        };
        let mut out = vec![];
        for spec in &schema.props {
            match props.get(&spec.name) {
                None if spec.required && spec.default.is_none() => {
                    out.push(SchemaViolation::MissingRequiredProp {
                        label_or_type: label.into(),
                        prop: spec.name.clone(),
                    });
                }
                Some(v) if !type_matches(v, &spec.ty) => {
                    out.push(SchemaViolation::WrongPropType {
                        label_or_type: label.into(),
                        prop: spec.name.clone(),
                        expected: spec.ty.name(),
                        got: value_type_name(v).into(),
                    });
                }
                _ => {}
            }
        }
        out
    }

    /// Validate rel endpoints. `start_label`/`end_label` are `None` if the
    /// referenced node is server-side and label was not provided.
    pub fn validate_rel_endpoints(
        &self,
        r#type: &str,
        start_label: Option<&str>,
        end_label: Option<&str>,
    ) -> Vec<SchemaViolation> {
        let Some(schema) = self.rels.get(r#type) else {
            return vec![SchemaViolation::UnknownRelType { r#type: r#type.into() }];
        };
        let (Some(s), Some(e)) = (start_label, end_label) else { return vec![]; };
        if schema.allowed_endpoints.is_empty() { return vec![]; }
        if schema.allowed_endpoints.iter().any(|(a, b)| a == s && b == e) {
            vec![]
        } else {
            vec![SchemaViolation::DisallowedEndpoints {
                r#type: r#type.into(),
                start: s.into(),
                end: e.into(),
            }]
        }
    }
}

fn type_matches(v: &PropValue, t: &PropType) -> bool {
    use PropType::*;
    match (v, t) {
        (PropValue::Null, _) => true, // null allowed at type level; required handles presence
        (PropValue::Bool(_), Bool) => true,
        (PropValue::Int(_), Int) => true,
        (PropValue::Float(_), Float) => true,
        (PropValue::String(_), String) => true,
        (PropValue::Bytes(_), Bytes) => true,
        (PropValue::Date(_), Date) => true,
        (PropValue::DateTime(_), DateTime) => true,
        (PropValue::Duration { .. }, Duration) => true,
        (PropValue::Point2D { .. }, Point) => true,
        (PropValue::Map(_), Map) => true,
        (PropValue::List(xs), List(inner)) => xs.iter().all(|x| type_matches(x, inner)),
        _ => false,
    }
}

fn value_type_name(v: &PropValue) -> &'static str {
    match v {
        PropValue::Null => "Null",
        PropValue::Bool(_) => "Bool",
        PropValue::Int(_) => "Int",
        PropValue::Float(_) => "Float",
        PropValue::String(_) => "String",
        PropValue::Bytes(_) => "Bytes",
        PropValue::List(_) => "List",
        PropValue::Map(_) => "Map",
        PropValue::Date(_) => "Date",
        PropValue::DateTime(_) => "DateTime",
        PropValue::Duration { .. } => "Duration",
        PropValue::Point2D { .. } => "Point",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SchemaViolation;
    use crate::value::PropValue;
    use std::collections::BTreeMap;

    fn reg() -> SchemaRegistry {
        let mut r = SchemaRegistry::new();
        r.add_node(types::NodeSchema::builder("Person")
            .prop("name", types::PropType::String).required()
            .prop("age", types::PropType::Int)
            .build());
        r.add_rel(types::RelSchema::builder("KNOWS")
            .endpoints("Person", "Person")
            .build());
        r
    }

    #[test]
    fn missing_required_prop() {
        let r = reg();
        let mut props = BTreeMap::new();
        props.insert("age".into(), PropValue::Int(30));
        let v = r.validate_node_props("Person", &props);
        assert!(v.iter().any(|x| matches!(x,
            SchemaViolation::MissingRequiredProp { prop, .. } if prop == "name")));
    }

    #[test]
    fn wrong_prop_type() {
        let r = reg();
        let mut props = BTreeMap::new();
        props.insert("name".into(), PropValue::String("a".into()));
        props.insert("age".into(), PropValue::String("not int".into()));
        let v = r.validate_node_props("Person", &props);
        assert!(v.iter().any(|x| matches!(x, SchemaViolation::WrongPropType { .. })));
    }

    #[test]
    fn unknown_label() {
        let r = reg();
        let v = r.validate_node_props("Alien", &BTreeMap::new());
        assert!(v.iter().any(|x| matches!(x, SchemaViolation::UnknownLabel { .. })));
    }

    #[test]
    fn endpoints_allowed() {
        let r = reg();
        let v = r.validate_rel_endpoints("KNOWS", Some("Person"), Some("Person"));
        assert!(v.is_empty());
        let v = r.validate_rel_endpoints("KNOWS", Some("Person"), Some("Org"));
        assert!(v.iter().any(|x| matches!(x, SchemaViolation::DisallowedEndpoints { .. })));
    }
}
