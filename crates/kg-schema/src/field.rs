//! FieldSpec + FieldType — richer than kg-core::schema::PropType because
//! it carries enum value lists and ref-to-label info needed by the UI.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldType {
    String,
    Int,
    Float,
    Bool,
    Date,
    DateTime,
    Enum { values: Vec<String> },
    #[serde(rename = "ref")]
    Ref { label: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldSpec {
    pub name: String,
    #[serde(flatten)]
    pub ty: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}
