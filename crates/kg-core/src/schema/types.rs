//! Schema definition types.

use crate::value::PropValue;
use serde::{Deserialize, Serialize};

/// Cardinality hint (used for documentation + future validation).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cardinality {
    OneToOne,
    OneToMany,
    #[default]
    ManyToMany,
}

/// Allowed value type for a property.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropType {
    Bool, Int, Float, String, Bytes,
    Date, DateTime, Duration, Point,
    List(Box<PropType>),
    Map,
}

impl PropType {
    /// Human-readable name (for error messages).
    pub fn name(&self) -> String {
        match self {
            PropType::Bool => "Bool".into(),
            PropType::Int  => "Int".into(),
            PropType::Float => "Float".into(),
            PropType::String => "String".into(),
            PropType::Bytes => "Bytes".into(),
            PropType::Date => "Date".into(),
            PropType::DateTime => "DateTime".into(),
            PropType::Duration => "Duration".into(),
            PropType::Point => "Point".into(),
            PropType::List(inner) => format!("List<{}>", inner.name()),
            PropType::Map => "Map".into(),
        }
    }
}

/// A single property specification.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropSpec {
    pub name: String,
    pub ty: PropType,
    pub required: bool,
    pub default: Option<PropValue>,
}

/// Node schema (label + props + uniqueness + indexes).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeSchema {
    pub label: String,
    pub props: Vec<PropSpec>,
    pub uniqueness: Vec<Vec<String>>,
    pub indexes: Vec<Vec<String>>,
}

impl NodeSchema {
    pub fn builder(label: impl Into<String>) -> NodeSchemaBuilder {
        NodeSchemaBuilder {
            inner: NodeSchema {
                label: label.into(),
                props: vec![],
                uniqueness: vec![],
                indexes: vec![],
            },
            cursor: None,
        }
    }
}

pub struct NodeSchemaBuilder {
    inner: NodeSchema,
    cursor: Option<usize>,
}

impl NodeSchemaBuilder {
    pub fn prop(mut self, name: impl Into<String>, ty: PropType) -> Self {
        self.inner.props.push(PropSpec { name: name.into(), ty, required: false, default: None });
        self.cursor = Some(self.inner.props.len() - 1);
        self
    }
    pub fn required(mut self) -> Self {
        if let Some(i) = self.cursor { self.inner.props[i].required = true; }
        self
    }
    pub fn default(mut self, v: PropValue) -> Self {
        if let Some(i) = self.cursor { self.inner.props[i].default = Some(v); }
        self
    }
    pub fn unique<I, S>(mut self, props: I) -> Self
    where I: IntoIterator<Item = S>, S: Into<String> {
        self.inner.uniqueness.push(props.into_iter().map(Into::into).collect());
        self
    }
    pub fn index<I, S>(mut self, props: I) -> Self
    where I: IntoIterator<Item = S>, S: Into<String> {
        self.inner.indexes.push(props.into_iter().map(Into::into).collect());
        self
    }
    pub fn build(self) -> NodeSchema { self.inner }
}

/// Rel schema (type + allowed endpoints + props + cardinality).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RelSchema {
    pub r#type: String,
    pub allowed_endpoints: Vec<(String, String)>,
    pub props: Vec<PropSpec>,
    pub cardinality: Cardinality,
}

impl RelSchema {
    pub fn builder(r#type: impl Into<String>) -> RelSchemaBuilder {
        RelSchemaBuilder {
            inner: RelSchema {
                r#type: r#type.into(),
                allowed_endpoints: vec![],
                props: vec![],
                cardinality: Cardinality::ManyToMany,
            },
            cursor: None,
        }
    }
}

pub struct RelSchemaBuilder {
    inner: RelSchema,
    cursor: Option<usize>,
}

impl RelSchemaBuilder {
    pub fn endpoints(mut self, start: impl Into<String>, end: impl Into<String>) -> Self {
        self.inner.allowed_endpoints.push((start.into(), end.into()));
        self
    }
    pub fn prop(mut self, name: impl Into<String>, ty: PropType) -> Self {
        self.inner.props.push(PropSpec { name: name.into(), ty, required: false, default: None });
        self.cursor = Some(self.inner.props.len() - 1);
        self
    }
    pub fn required(mut self) -> Self {
        if let Some(i) = self.cursor { self.inner.props[i].required = true; }
        self
    }
    pub fn cardinality(mut self, c: Cardinality) -> Self { self.inner.cardinality = c; self }
    pub fn build(self) -> RelSchema { self.inner }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_schema_builder() {
        let s = NodeSchema::builder("Person")
            .prop("name", PropType::String).required()
            .prop("age", PropType::Int).default(crate::value::PropValue::Int(0))
            .unique(["name"])
            .build();
        assert_eq!(s.label, "Person");
        assert_eq!(s.props.len(), 2);
        assert!(s.props.iter().any(|p| p.name == "name" && p.required));
        assert_eq!(s.uniqueness, vec![vec!["name".to_string()]]);
    }

    #[test]
    fn rel_schema_builder() {
        let s = RelSchema::builder("KNOWS")
            .endpoints("Person", "Person")
            .endpoints("Person", "Org")
            .prop("since", PropType::Date).build();
        assert_eq!(s.r#type, "KNOWS");
        assert_eq!(s.allowed_endpoints.len(), 2);
        assert_eq!(s.cardinality, Cardinality::ManyToMany);
    }
}
