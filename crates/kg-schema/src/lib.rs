//! YAML-driven schema loader.

pub mod error;
pub mod field;

pub use error::SchemaError;
pub use field::{FieldSpec, FieldType};
