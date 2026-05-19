#![cfg(feature = "wasm")]

use crate::error::Neo4jError;
use kg_core::value::PropValue;
use serde_json::{json, Value};

/// Convert to Neo4j HTTP Query API v2 JSON value form.
/// API encodes typed values as `{ "$type": "...", "_value": "..." }`.
#[allow(dead_code)]
pub fn to_json(v: &PropValue) -> Result<Value, Neo4jError> {
    use PropValue::*;
    Ok(match v {
        Null     => Value::Null,
        Bool(b)  => Value::Bool(*b),
        Int(i)   => json!({"$type": "Integer", "_value": i.to_string()}),
        Float(f) => json!({"$type": "Float",   "_value": f.to_string()}),
        String(s) => json!({"$type": "String", "_value": s}),
        Bytes(b) => json!({"$type": "Base64",  "_value": base64_encode(b)}),
        List(xs) => {
            let inner: Vec<Value> = xs.iter().map(to_json).collect::<Result<_, _>>()?;
            json!({"$type": "List", "_value": inner})
        }
        Map(m) => {
            let mut obj = serde_json::Map::new();
            for (k, vv) in m { obj.insert(k.clone(), to_json(vv)?); }
            json!({"$type": "Map", "_value": Value::Object(obj)})
        }
        Date(d) => json!({"$type": "Date",     "_value": d.format("%Y-%m-%d").to_string()}),
        DateTime(dt) => json!({"$type": "OffsetDateTime", "_value": dt.to_rfc3339()}),
        Duration { seconds, nanos } => json!({
            "$type": "Duration",
            "_value": format!("PT{seconds}.{nanos:09}S")
        }),
        Point2D { srid, x, y } => json!({
            "$type": "Point",
            "_value": {"srid": srid, "coordinates": [x, y]}
        }),
    })
}

#[allow(dead_code)]
pub fn from_json(j: &Value) -> Result<PropValue, Neo4jError> {
    if j.is_null() { return Ok(PropValue::Null); }
    if let Some(b) = j.as_bool() { return Ok(PropValue::Bool(b)); }
    if let Some(obj) = j.as_object() {
        let ty = obj.get("$type").and_then(|v| v.as_str()).ok_or_else(|| Neo4jError::Conversion {
            from: "json", to: "PropValue", reason: "missing $type".into(),
        })?;
        let val = obj.get("_value").ok_or_else(|| Neo4jError::Conversion {
            from: "json", to: "PropValue", reason: "missing _value".into(),
        })?;
        return Ok(match ty {
            "Integer" => PropValue::Int(val.as_str().and_then(|s| s.parse().ok())
                .ok_or_else(|| conv("Integer"))?),
            "Float"   => PropValue::Float(val.as_str().and_then(|s| s.parse().ok())
                .ok_or_else(|| conv("Float"))?),
            "String"  => PropValue::String(val.as_str().ok_or_else(|| conv("String"))?.into()),
            "Date"    => PropValue::Date(chrono::NaiveDate::parse_from_str(
                val.as_str().ok_or_else(|| conv("Date"))?, "%Y-%m-%d",
            ).map_err(|e| Neo4jError::Conversion { from: "json", to: "Date", reason: e.to_string() })?),
            "OffsetDateTime" => PropValue::DateTime(
                chrono::DateTime::parse_from_rfc3339(val.as_str().ok_or_else(|| conv("OffsetDateTime"))?)
                    .map_err(|e| Neo4jError::Conversion { from: "json", to: "DateTime", reason: e.to_string() })?
            ),
            "List" => {
                let arr = val.as_array().ok_or_else(|| conv("List"))?;
                PropValue::List(arr.iter().map(from_json).collect::<Result<_, _>>()?)
            }
            "Map" => {
                let m = val.as_object().ok_or_else(|| conv("Map"))?;
                let mut out = std::collections::BTreeMap::new();
                for (k, v) in m { out.insert(k.clone(), from_json(v)?); }
                PropValue::Map(out)
            }
            other => return Err(Neo4jError::Conversion {
                from: "json", to: "PropValue", reason: format!("unsupported type tag: {other}"),
            }),
        });
    }
    Err(Neo4jError::Conversion { from: "json", to: "PropValue", reason: "unexpected shape".into() })
}

#[allow(dead_code)]
fn conv(t: &'static str) -> Neo4jError {
    Neo4jError::Conversion { from: "json", to: t, reason: "invalid value".into() }
}

#[allow(dead_code)]
fn base64_encode(b: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_roundtrip() {
        let v = PropValue::Int(42);
        assert_eq!(v, from_json(&to_json(&v).unwrap()).unwrap());
    }
    #[test]
    fn string_roundtrip() {
        let v = PropValue::String("hi".into());
        assert_eq!(v, from_json(&to_json(&v).unwrap()).unwrap());
    }
    #[test]
    fn list_roundtrip() {
        let v = PropValue::List(vec![PropValue::Int(1), PropValue::String("x".into())]);
        assert_eq!(v, from_json(&to_json(&v).unwrap()).unwrap());
    }
}
