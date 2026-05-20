use axum::{extract::{Path, Query, State}, Json};
use kg_core::value::PropValue;
use kg_schema::FieldType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateBody {
    pub label: String,
    pub props: BTreeMap<String, serde_json::Value>,
}

#[derive(Serialize)]
pub struct CreateResponse {
    pub id: i64,
    pub label: String,
    pub props: BTreeMap<String, serde_json::Value>,
}

pub async fn create(
    State(s): State<AppState>,
    Json(body): Json<CreateBody>,
) -> Result<Json<CreateResponse>, ApiError> {
    let node_def = s.schema.nodes.get(&body.label).ok_or_else(|| ApiError {
        code: "400.unknown_label".into(),
        message: format!("unknown node label `{}`", body.label),
    })?;
    validate_props(&node_def.props, &body.props)?;

    let mut props: BTreeMap<String, PropValue> = BTreeMap::new();
    for (k, v) in &body.props {
        props.insert(k.clone(), json_to_prop(v.clone())?);
    }

    // Sanitize label for use in Cypher.
    if !body.label.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ApiError {
            code: "400.bad_label".into(),
            message: "label has illegal chars".into(),
        });
    }

    let cypher = format!(
        "CREATE (n:`{}`) SET n = $props RETURN id(n) AS id, properties(n) AS props",
        body.label
    );
    let rows = s
        .client
        .query::<BTreeMap<String, PropValue>>(
            &cypher,
            [("props", PropValue::Map(props))],
        )
        .await
        .map_err(ApiError::from)?;
    let row = rows.into_iter().next().ok_or_else(|| ApiError {
        code: "500.no_row".into(),
        message: "CREATE returned no row".into(),
    })?;
    let id = match row.get("id") {
        Some(PropValue::Int(i)) => *i,
        _ => return Err(ApiError { code: "500.bad_id".into(), message: "no id".into() }),
    };
    let props_json: BTreeMap<String, serde_json::Value> = match row.get("props") {
        Some(PropValue::Map(m)) => m.iter().map(|(k, v)| (k.clone(), prop_to_json(v))).collect(),
        _ => BTreeMap::new(),
    };
    Ok(Json(CreateResponse {
        id,
        label: body.label,
        props: props_json,
    }))
}

pub(crate) fn validate_props(
    specs: &[kg_schema::FieldSpec],
    props: &BTreeMap<String, serde_json::Value>,
) -> Result<(), ApiError> {
    let mut errors = vec![];
    for spec in specs {
        match props.get(&spec.name) {
            None if spec.required => {
                errors.push(format!("missing required field `{}`", spec.name));
            }
            Some(v) if !type_matches(v, &spec.ty) => {
                errors.push(format!("field `{}` wrong type for spec", spec.name));
            }
            _ => {}
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ApiError {
            code: "400.validation".into(),
            message: errors.join("; "),
        })
    }
}

pub(crate) fn type_matches(v: &serde_json::Value, ty: &FieldType) -> bool {
    use serde_json::Value as J;
    match (v, ty) {
        (J::Null, _) => true,
        (J::Bool(_), FieldType::Bool) => true,
        (J::Number(n), FieldType::Int) => n.is_i64() || n.is_u64(),
        (J::Number(_), FieldType::Float) => true,
        (J::String(_), FieldType::String | FieldType::Date | FieldType::DateTime) => true,
        (J::String(s), FieldType::Enum { values }) => values.iter().any(|x| x == s),
        (J::Number(_), FieldType::Ref { .. }) => true,
        _ => false,
    }
}

pub(crate) fn json_to_prop(v: serde_json::Value) -> Result<PropValue, ApiError> {
    use serde_json::Value as J;
    Ok(match v {
        J::Null => PropValue::Null,
        J::Bool(b) => PropValue::Bool(b),
        J::Number(n) => {
            if let Some(i) = n.as_i64() {
                PropValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                PropValue::Float(f)
            } else {
                return Err(ApiError {
                    code: "400.bad_num".into(),
                    message: format!("{n}"),
                });
            }
        }
        J::String(s) => PropValue::String(s),
        J::Array(a) => PropValue::List(
            a.into_iter()
                .map(json_to_prop)
                .collect::<Result<_, _>>()?,
        ),
        J::Object(o) => {
            let mut m = BTreeMap::new();
            for (k, x) in o {
                m.insert(k, json_to_prop(x)?);
            }
            PropValue::Map(m)
        }
    })
}

// ── PUT /entities/:id ────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UpdateBody {
    pub set: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub unset: Vec<String>,
}

pub async fn update(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Find label first to validate.
    let label_rows = s
        .client
        .query::<BTreeMap<String, PropValue>>(
            "MATCH (n) WHERE id(n) = $id RETURN labels(n) AS labels",
            [("id", PropValue::Int(id))],
        )
        .await
        .map_err(ApiError::from)?;
    let row = label_rows.into_iter().next().ok_or_else(|| ApiError {
        code: "404.not_found".into(),
        message: format!("no node with id {id}"),
    })?;
    let label = match row.get("labels") {
        Some(PropValue::List(xs)) => xs.iter().find_map(|x| match x {
            PropValue::String(s) => Some(s.clone()),
            _ => None,
        }),
        _ => None,
    };

    if let Some(label) = label.as_deref() {
        if let Some(def) = s.schema.nodes.get(label) {
            let mut errs = vec![];
            for (k, v) in &body.set {
                if let Some(spec) = def.props.iter().find(|p| p.name == *k) {
                    if !type_matches(v, &spec.ty) {
                        errs.push(format!("field `{}` wrong type", k));
                    }
                }
            }
            if !errs.is_empty() {
                return Err(ApiError {
                    code: "400.validation".into(),
                    message: errs.join("; "),
                });
            }
        }
    }

    let mut sets = String::new();
    let mut params: Vec<(String, PropValue)> = vec![("id".into(), PropValue::Int(id))];
    let mut i = 0;
    for (k, v) in &body.set {
        // Sanitize prop key (allow only alphanumerics + underscore).
        if !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(ApiError {
                code: "400.bad_key".into(),
                message: format!("illegal key `{k}`"),
            });
        }
        let pkey = format!("p_{i}");
        sets.push_str(&format!(" SET n.`{k}` = ${pkey}"));
        params.push((pkey, json_to_prop(v.clone())?));
        i += 1;
    }
    for k in &body.unset {
        if !k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(ApiError {
                code: "400.bad_key".into(),
                message: format!("illegal key `{k}`"),
            });
        }
        sets.push_str(&format!(" REMOVE n.`{k}`"));
    }
    let cypher = format!("MATCH (n) WHERE id(n) = $id{sets} RETURN id(n) AS id");
    let _ = s
        .client
        .query::<BTreeMap<String, PropValue>>(&cypher, params)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

pub(crate) fn prop_to_json(v: &PropValue) -> serde_json::Value {
    use PropValue::*;
    match v {
        Null => serde_json::Value::Null,
        Bool(b) => serde_json::Value::Bool(*b),
        Int(i) => serde_json::json!(i),
        Float(f) => serde_json::json!(f),
        String(s) => serde_json::Value::String(s.clone()),
        Bytes(_) => serde_json::Value::Null,
        List(xs) => serde_json::Value::Array(xs.iter().map(prop_to_json).collect()),
        Map(m) => serde_json::Value::Object(
            m.iter().map(|(k, v)| (k.clone(), prop_to_json(v))).collect(),
        ),
        Date(d) => serde_json::Value::String(d.format("%Y-%m-%d").to_string()),
        DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
        Duration { seconds, nanos } => serde_json::json!({"seconds": seconds, "nanos": nanos}),
        Point2D { srid, x, y } => serde_json::json!({"srid": srid, "x": x, "y": y}),
    }
}

// ── DELETE /entities/:id ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DeleteParams {
    #[serde(default)]
    pub cascade: bool,
}

pub async fn delete(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    Query(p): Query<DeleteParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let cypher = if p.cascade {
        "MATCH (n) WHERE id(n) = $id DETACH DELETE n"
    } else {
        "MATCH (n) WHERE id(n) = $id DELETE n"
    };
    s.client.query::<BTreeMap<String, PropValue>>(cypher, [("id", PropValue::Int(id))]).await.map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

// ── GET /entities/:id ────────────────────────────────────────────────────────

pub async fn get_one(
    State(s): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows = s.client.query::<BTreeMap<String, PropValue>>(
        "MATCH (n) WHERE id(n) = $id \
         OPTIONAL MATCH (n)-[r_out]->(t) \
         OPTIONAL MATCH (src)-[r_in]->(n) \
         RETURN id(n) AS id, labels(n) AS labels, properties(n) AS props, \
                collect(DISTINCT {id: id(r_out), type: type(r_out), target_id: id(t), target_labels: labels(t), props: properties(r_out)}) AS out_rels, \
                collect(DISTINCT {id: id(r_in), type: type(r_in), source_id: id(src), source_labels: labels(src), props: properties(r_in)}) AS in_rels",
        [("id", PropValue::Int(id))],
    ).await.map_err(ApiError::from)?;
    let row = rows.into_iter().next().ok_or_else(|| ApiError {
        code: "404.not_found".into(), message: format!("no node with id {id}"),
    })?;

    // Filter out null-rel entries (caused by OPTIONAL MATCH with no match).
    let filter_rels = |v: serde_json::Value| -> serde_json::Value {
        if let serde_json::Value::Array(arr) = v {
            serde_json::Value::Array(
                arr.into_iter().filter(|x| !matches!(x.get("id"), Some(serde_json::Value::Null) | None)).collect()
            )
        } else { v }
    };

    let body = serde_json::json!({
        "id": id,
        "labels": prop_to_json(row.get("labels").unwrap_or(&PropValue::Null)),
        "props": prop_to_json(row.get("props").unwrap_or(&PropValue::Null)),
        "out_rels": filter_rels(prop_to_json(row.get("out_rels").unwrap_or(&PropValue::Null))),
        "in_rels": filter_rels(prop_to_json(row.get("in_rels").unwrap_or(&PropValue::Null))),
    });
    Ok(Json(body))
}
