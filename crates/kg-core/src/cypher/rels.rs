//! Rel op emission.

use crate::cypher::nodes::node_var;
use crate::cypher::statement::{sanitize_ident, IdentError, Statement};
use crate::node::{LocalId, NodeRef};
use crate::rel::RelId;
use crate::uow::PropPatch;
use crate::value::PropValue;
use std::collections::BTreeMap;

pub fn rel_var(local: LocalId) -> String { format!("r_{}", local.0) }

struct Endpoint {
    var: String,
    /// MATCH prefix to be prepended, if endpoint is server-side.
    prelude: Option<String>,
    extra_params: Vec<(String, PropValue)>,
}

fn endpoint(nr: &NodeRef, role: &str, rel_local: LocalId) -> Endpoint {
    match nr {
        NodeRef::Local(l) => Endpoint {
            var: node_var(*l), prelude: None, extra_params: vec![],
        },
        NodeRef::Server(id) => {
            let var = format!("n_{role}_{}", rel_local.0);
            let key = format!("id_{role}_{}", rel_local.0);
            Endpoint {
                var: var.clone(),
                prelude: Some(format!("MATCH ({var}) WHERE id({var}) = ${key}")),
                extra_params: vec![(key, PropValue::Int(id.0))],
            }
        }
    }
}

pub fn emit_create_rel(
    local: LocalId,
    start: NodeRef,
    end: NodeRef,
    r#type: &str,
    props: &BTreeMap<String, PropValue>,
) -> Result<Statement, IdentError> {
    sanitize_ident(r#type)?;
    let s = endpoint(&start, "start", local);
    let e = endpoint(&end,   "end",   local);
    let rvar = rel_var(local);
    let pkey = format!("props_{rvar}");
    let mut preludes = vec![];
    let mut params: Vec<(String, PropValue)> = vec![];
    if let Some(p) = &s.prelude { preludes.push(p.clone()); }
    if let Some(p) = &e.prelude { preludes.push(p.clone()); }
    params.extend(s.extra_params);
    params.extend(e.extra_params);
    params.push((pkey.clone(), PropValue::Map(props.clone())));
    let prelude = preludes.join(" ");
    let body = format!(
        "CREATE ({})-[{rvar}:`{ty}` ${pkey}]->({})",
        s.var, e.var, ty = r#type,
    );
    let cypher = if prelude.is_empty() { body } else { format!("{prelude} {body}") };
    Ok(Statement::new(cypher, params))
}

pub fn emit_merge_rel(
    local: LocalId,
    start: NodeRef,
    end: NodeRef,
    r#type: &str,
    key_props: &BTreeMap<String, PropValue>,
    set_props: &BTreeMap<String, PropValue>,
) -> Result<Statement, IdentError> {
    sanitize_ident(r#type)?;
    let s = endpoint(&start, "start", local);
    let e = endpoint(&end,   "end",   local);
    let rvar = rel_var(local);
    let kkey = format!("kprops_{rvar}");
    let skey = format!("sprops_{rvar}");

    let key_pattern = if key_props.is_empty() {
        String::new()
    } else {
        let inner = key_props
            .keys()
            .map(|k| sanitize_ident(k).map(|s| format!("`{s}`: ${kkey}.`{s}`")))
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        format!(" {{ {inner} }}")
    };

    let mut preludes = vec![];
    let mut params: Vec<(String, PropValue)> = vec![];
    if let Some(p) = &s.prelude { preludes.push(p.clone()); }
    if let Some(p) = &e.prelude { preludes.push(p.clone()); }
    params.extend(s.extra_params);
    params.extend(e.extra_params);
    params.push((kkey.clone(), PropValue::Map(key_props.clone())));

    let mut body = format!(
        "MERGE ({})-[{rvar}:`{ty}`{key_pattern}]->({})",
        s.var, e.var, ty = r#type,
    );
    if !set_props.is_empty() {
        body.push_str(&format!(" SET {rvar} += ${skey}"));
        params.push((skey, PropValue::Map(set_props.clone())));
    }
    let prelude = preludes.join(" ");
    let cypher = if prelude.is_empty() { body } else { format!("{prelude} {body}") };
    Ok(Statement::new(cypher, params))
}

pub fn emit_update_rel(id: RelId, patch: &PropPatch) -> Result<Statement, IdentError> {
    let var = format!("r_upd_{}", id.0);
    let rkey = format!("rid_{}", id.0);
    let mut sets = vec![];
    let mut removes = vec![];
    let mut params: Vec<(String, PropValue)> = vec![(rkey.clone(), PropValue::Int(id.0))];
    for (name, val) in patch.entries() {
        let n = sanitize_ident(name)?;
        match val {
            Some(v) => {
                let pkey = format!("set_{}_{}", id.0, n);
                sets.push(format!("SET {var}.`{n}` = ${pkey}"));
                params.push((pkey, v.clone()));
            }
            None => removes.push(format!("REMOVE {var}.`{n}`")),
        }
    }
    let body = sets.into_iter().chain(removes).collect::<Vec<_>>().join(" ");
    Ok(Statement::new(
        format!("MATCH ()-[{var}]->() WHERE id({var}) = ${rkey} {body}"),
        params,
    ))
}

pub fn emit_delete_rel(id: RelId) -> Result<Statement, IdentError> {
    let var = format!("r_del_{}", id.0);
    let rkey = format!("rid_{}", id.0);
    Ok(Statement::new(
        format!("MATCH ()-[{var}]->() WHERE id({var}) = ${rkey} DELETE {var}"),
        [(rkey, PropValue::Int(id.0))],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{LocalId, NodeId, NodeRef};
    use crate::rel::RelId;
    use crate::uow::PropPatch;
    use crate::value::PropValue;
    use std::collections::BTreeMap;

    #[test]
    fn rel_between_two_locals_uses_vars() {
        let s = emit_create_rel(
            LocalId(1),
            NodeRef::Local(LocalId(2)),
            NodeRef::Local(LocalId(3)),
            "KNOWS",
            &BTreeMap::new(),
        ).unwrap();
        assert!(s.cypher.contains("(n_2)-[r_1:`KNOWS`"));
        assert!(s.cypher.contains("->(n_3)"));
        assert!(!s.cypher.contains("MATCH"));
    }

    #[test]
    fn rel_with_server_endpoint_emits_match() {
        let s = emit_create_rel(
            LocalId(1),
            NodeRef::Local(LocalId(2)),
            NodeRef::Server(NodeId(99)),
            "KNOWS",
            &BTreeMap::new(),
        ).unwrap();
        assert!(s.cypher.contains("MATCH (n_end_1) WHERE id(n_end_1) = $id_end_1"));
        assert_eq!(s.params.get("id_end_1"), Some(&PropValue::Int(99)));
    }

    #[test]
    fn rel_props_param() {
        let mut props = BTreeMap::new();
        props.insert("since".into(), PropValue::Int(2020));
        let s = emit_create_rel(
            LocalId(7),
            NodeRef::Local(LocalId(1)),
            NodeRef::Local(LocalId(2)),
            "KNOWS",
            &props,
        ).unwrap();
        assert!(s.cypher.contains("$props_r_7"));
        assert_eq!(s.params.get("props_r_7"), Some(&PropValue::Map(props)));
    }

    #[test]
    fn update_rel_emits_match_id() {
        let s = emit_update_rel(RelId(5),
            &PropPatch::new().set("a", PropValue::Int(1))).unwrap();
        assert!(s.cypher.contains("MATCH ()-[r_upd_5]->() WHERE id(r_upd_5) = $rid_5"));
        assert!(s.cypher.contains("SET r_upd_5.`a` = $set_5_a"));
    }

    #[test]
    fn delete_rel_test() {
        let s = emit_delete_rel(RelId(8)).unwrap();
        assert!(s.cypher.contains("MATCH ()-[r_del_8]->() WHERE id(r_del_8) = $rid_8 DELETE r_del_8"));
    }
}
