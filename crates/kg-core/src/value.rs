//! Property values (Bolt-compatible scalar/composite types).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A property value compatible with Neo4j Bolt scalar and composite types.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PropValue {
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<PropValue>),
    Map(BTreeMap<String, PropValue>),
    Date(chrono::NaiveDate),
    DateTime(chrono::DateTime<chrono::FixedOffset>),
    Duration { seconds: i64, nanos: i32 },
    Point2D { srid: i32, x: f64, y: f64 },
}

impl PropValue {
    /// Convenience constructor for `Map`.
    pub fn map_of<I, K>(items: I) -> Self
    where
        I: IntoIterator<Item = (K, PropValue)>,
        K: Into<String>,
    {
        PropValue::Map(items.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}

macro_rules! impl_from_scalar {
    ($($t:ty => $var:ident),* $(,)?) => {
        $(impl From<$t> for PropValue {
            fn from(v: $t) -> Self { PropValue::$var(v.into()) }
        })*
    };
}

impl_from_scalar!(
    bool   => Bool,
    i32    => Int,
    i64    => Int,
    f32    => Float,
    f64    => Float,
    String => String,
);

impl From<&str> for PropValue {
    fn from(v: &str) -> Self { PropValue::String(v.to_owned()) }
}

impl From<Vec<PropValue>> for PropValue {
    fn from(v: Vec<PropValue>) -> Self { PropValue::List(v) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn from_primitives() {
        assert_eq!(PropValue::from(true), PropValue::Bool(true));
        assert_eq!(PropValue::from(42_i64), PropValue::Int(42));
        assert_eq!(PropValue::from(1.5_f64), PropValue::Float(1.5));
        assert_eq!(PropValue::from("hi"), PropValue::String("hi".into()));
        assert_eq!(PropValue::from(String::from("hi")), PropValue::String("hi".into()));
    }

    #[test]
    fn list_and_map() {
        let v = PropValue::from(vec![PropValue::Int(1), PropValue::Int(2)]);
        assert!(matches!(v, PropValue::List(ref xs) if xs.len() == 2));
        let mut m = std::collections::BTreeMap::new();
        m.insert("k".to_string(), PropValue::Int(1));
        let v = PropValue::Map(m);
        assert!(matches!(v, PropValue::Map(_)));
    }

    #[test]
    fn temporal() {
        let d = chrono::NaiveDate::from_ymd_opt(2026, 5, 19).unwrap();
        assert!(matches!(PropValue::Date(d), PropValue::Date(_)));
        let dt = chrono::FixedOffset::east_opt(0).unwrap()
            .with_ymd_and_hms(2026, 5, 19, 12, 0, 0).unwrap();
        assert!(matches!(PropValue::DateTime(dt), PropValue::DateTime(_)));
    }

    #[test]
    fn null_default() {
        assert_eq!(PropValue::default(), PropValue::Null);
    }
}
