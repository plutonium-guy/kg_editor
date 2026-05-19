#![cfg(feature = "native")]

use crate::error::Neo4jError;
use kg_core::value::PropValue;
use neo4rs::{
    BoltBoolean, BoltFloat, BoltInteger, BoltList, BoltMap, BoltNull, BoltString, BoltType,
};

#[allow(dead_code)]
pub fn to_bolt(v: &PropValue) -> Result<BoltType, Neo4jError> {
    use PropValue::*;
    Ok(match v {
        Null => BoltType::Null(BoltNull),
        Bool(b) => BoltType::Boolean(BoltBoolean::new(*b)),
        Int(i) => BoltType::Integer(BoltInteger::new(*i)),
        Float(f) => BoltType::Float(BoltFloat::new(*f)),
        String(s) => BoltType::String(BoltString::new(s)),
        Bytes(_) => {
            return Err(Neo4jError::Conversion {
                from: "Bytes",
                to: "BoltType",
                reason: "phase-0 unsupported".into(),
            })
        }
        List(xs) => {
            let mut list = BoltList::with_capacity(xs.len());
            for x in xs {
                list.push(to_bolt(x)?);
            }
            BoltType::List(list)
        }
        Map(m) => {
            let mut map = BoltMap::with_capacity(m.len());
            for (k, val) in m {
                map.put(BoltString::new(k), to_bolt(val)?);
            }
            BoltType::Map(map)
        }
        Date(_) | DateTime(_) | Duration { .. } | Point2D { .. } => {
            return Err(Neo4jError::Conversion {
                from: "temporal/spatial",
                to: "BoltType",
                reason: "phase-0 partial; tracked".into(),
            });
        }
    })
}

#[allow(dead_code)]
pub fn from_bolt(b: &BoltType) -> Result<PropValue, Neo4jError> {
    Ok(match b {
        BoltType::Null(_) => PropValue::Null,
        BoltType::Boolean(v) => PropValue::Bool(v.value),
        BoltType::Integer(v) => PropValue::Int(v.value),
        BoltType::Float(v) => PropValue::Float(v.value),
        BoltType::String(v) => PropValue::String(v.value.clone()),
        BoltType::List(v) => {
            let mut out = Vec::with_capacity(v.len());
            for x in v.iter() {
                out.push(from_bolt(x)?);
            }
            PropValue::List(out)
        }
        BoltType::Map(v) => {
            let mut out = std::collections::BTreeMap::new();
            for (k, val) in &v.value {
                out.insert(k.value.clone(), from_bolt(val)?);
            }
            PropValue::Map(out)
        }
        other => {
            return Err(Neo4jError::Conversion {
                from: "BoltType",
                to: "PropValue",
                reason: format!("unsupported variant: {other:?}"),
            })
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalars_roundtrip() {
        for v in [
            PropValue::Null,
            PropValue::Bool(true),
            PropValue::Int(42),
            PropValue::Float(1.5),
            PropValue::String("hi".into()),
        ] {
            let b = to_bolt(&v).unwrap();
            let back = from_bolt(&b).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn list_roundtrip() {
        let v = PropValue::List(vec![PropValue::Int(1), PropValue::String("x".into())]);
        let b = to_bolt(&v).unwrap();
        assert_eq!(v, from_bolt(&b).unwrap());
    }

    #[test]
    fn map_roundtrip() {
        let mut m = std::collections::BTreeMap::new();
        m.insert("k".into(), PropValue::Int(1));
        let v = PropValue::Map(m);
        let b = to_bolt(&v).unwrap();
        assert_eq!(v, from_bolt(&b).unwrap());
    }
}
