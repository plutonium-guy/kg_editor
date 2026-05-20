use axum::{extract::{Path, State}, Json};
use kg_core::value::PropValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use crate::error::ApiError;
use crate::routes::entities::json_to_prop;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateLink {
    pub r#type: String,
    pub start_id: i64,
    pub end_id: i64,
    #[serde(default)]
    pub props: BTreeMap<String, serde_json::Value>,
}

#[derive(Serialize)]
pub struct CreateLinkResponse {
    pub id: i64,
}

pub async fn create(
    State(s): State<AppState>,
    Json(body): Json<CreateLink>,
) -> Result<Json<CreateLinkResponse>, ApiError> {
    if !body.r#type.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ApiError { code: "400.bad_type".into(), message: "rel type has illegal chars".into() });
    }

    // Endpoint validation against schema (best-effort).
    if let Some(rel_def) = s.schema.rels.get(&body.r#type) {
        if !rel_def.endpoints.is_empty() {
            let label_rows = s.client.query::<BTreeMap<String, PropValue>>(
                "MATCH (s) WHERE id(s) = $sid MATCH (e) WHERE id(e) = $eid RETURN labels(s) AS sl, labels(e) AS el",
                vec![
                    ("sid".to_string(), PropValue::Int(body.start_id)),
                    ("eid".to_string(), PropValue::Int(body.end_id)),
                ],
            ).await.map_err(ApiError::from)?;
            if let Some(row) = label_rows.into_iter().next() {
                let sl = match row.get("sl") {
                    Some(PropValue::List(xs)) => xs.iter().find_map(|x| match x { PropValue::String(s) => Some(s.clone()), _ => None }),
                    _ => None,
                };
                let el = match row.get("el") {
                    Some(PropValue::List(xs)) => xs.iter().find_map(|x| match x { PropValue::String(s) => Some(s.clone()), _ => None }),
                    _ => None,
                };
                if let (Some(s), Some(e)) = (sl, el) {
                    if !rel_def.endpoints.iter().any(|(a, b)| a == &s && b == &e) {
                        return Err(ApiError {
                            code: "400.endpoint_not_allowed".into(),
                            message: format!("rel `{}` not allowed from `{s}` to `{e}`", body.r#type),
                        });
                    }
                }
            }
        }
    }

    let mut props: BTreeMap<String, PropValue> = BTreeMap::new();
    for (k, v) in body.props { props.insert(k, json_to_prop(v)?); }
    let cypher = format!(
        "MATCH (src) WHERE id(src) = $sid MATCH (dst) WHERE id(dst) = $eid CREATE (src)-[r:`{}` $p]->(dst) RETURN id(r) AS id",
        body.r#type
    );
    let rows = s.client.query::<BTreeMap<String, PropValue>>(
        &cypher,
        vec![
            ("sid".to_string(), PropValue::Int(body.start_id)),
            ("eid".to_string(), PropValue::Int(body.end_id)),
            ("p".to_string(),   PropValue::Map(props)),
        ],
    ).await.map_err(ApiError::from)?;
    let row = rows.into_iter().next().ok_or_else(|| ApiError {
        code: "500.no_row".into(), message: "no row".into(),
    })?;
    let id = match row.get("id") {
        Some(PropValue::Int(i)) => *i,
        _ => return Err(ApiError { code: "500.bad_id".into(), message: "no id".into() }),
    };
    Ok(Json(CreateLinkResponse { id }))
}

pub async fn delete(
    State(s): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    s.client.query::<BTreeMap<String, PropValue>>(
        "MATCH ()-[r]->() WHERE id(r) = $id DELETE r",
        vec![("id".to_string(), PropValue::Int(id))],
    ).await.map_err(ApiError::from)?;
    Ok(Json(serde_json::json!({"ok": true})))
}
