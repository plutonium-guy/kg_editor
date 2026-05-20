//! Node op emission.

use crate::cypher::statement::{sanitize_ident, IdentError, Statement};
use crate::node::{LocalId, NodeId};
use crate::uow::{CascadeRule, PropPatch};
use crate::value::PropValue;
use smallvec::SmallVec;
use std::collections::BTreeMap;

pub fn node_var(local: LocalId) -> String { format!("n_{}", local.0) }

pub fn emit_create_node(
    local: LocalId,
    labels: &SmallVec<[String; 2]>,
    props: &BTreeMap<String, PropValue>,
) -> Result<Statement, IdentError> {
    let var = node_var(local);
    let labels_str = labels
        .iter()
        .map(|l| sanitize_ident(l).map(|s| format!(":`{s}`")))
        .collect::<Result<Vec<_>, _>>()?
        .join("");
    let pkey = format!("props_{var}");
    Ok(Statement::new(
        format!("CREATE ({var}{labels_str} ${pkey})"),
        [(pkey, PropValue::Map(props.clone()))],
    ))
}

pub fn emit_merge_node(
    local: LocalId,
    labels: &SmallVec<[String; 2]>,
    key_props: &BTreeMap<String, PropValue>,
    set_props: &BTreeMap<String, PropValue>,
) -> Result<Statement, IdentError> {
    let var = node_var(local);
    let labels_str = labels
        .iter()
        .map(|l| sanitize_ident(l).map(|s| format!(":`{s}`")))
        .collect::<Result<Vec<_>, _>>()?
        .join("");
    let kkey = format!("kprops_{var}");
    let skey = format!("sprops_{var}");
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
    let mut cypher = format!("MERGE ({var}{labels_str}{key_pattern})");
    let mut params = vec![
        (kkey, PropValue::Map(key_props.clone())),
    ];
    if !set_props.is_empty() {
        cypher.push_str(&format!(" SET {var} += ${skey}"));
        params.push((skey, PropValue::Map(set_props.clone())));
    }
    Ok(Statement::new(cypher, params))
}

pub fn emit_update_node(id: NodeId, patch: &PropPatch) -> Result<Statement, IdentError> {
    let var = format!("n_upd_{}", id.0);
    let id_key = format!("id_{}", id.0);
    let mut sets = vec![];
    let mut removes = vec![];
    let mut params: Vec<(String, PropValue)> = vec![(id_key.clone(), PropValue::Int(id.0))];
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
    let cypher = format!(
        "MATCH ({var}) WHERE id({var}) = ${id_key} {body}"
    );
    Ok(Statement::new(cypher, params))
}

pub fn emit_delete_node(id: NodeId, cascade: CascadeRule) -> Result<Statement, IdentError> {
    let var = format!("n_del_{}", id.0);
    let id_key = format!("id_{}", id.0);
    let action = match cascade {
        CascadeRule::Detach => format!("DETACH DELETE {var}"),
        CascadeRule::Strict => format!("DELETE {var}"),
    };
    Ok(Statement::new(
        format!("MATCH ({var}) WHERE id({var}) = ${id_key} {action}"),
        [(id_key, PropValue::Int(id.0))],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{LocalId, NodeId};
    use crate::uow::{CascadeRule, PropPatch};
    use crate::value::PropValue;
    use std::collections::BTreeMap;

    #[test]
    fn create_node_emits_create_with_param_props() {
        let mut props = BTreeMap::new();
        props.insert("name".into(), PropValue::String("A".into()));
        let s = emit_create_node(LocalId(1), &smallvec::smallvec!["Person".into()], &props).unwrap();
        assert!(s.cypher.contains("CREATE (n_1:`Person` $props_n_1)"));
        assert_eq!(s.params.get("props_n_1"), Some(&PropValue::Map(props.clone())));
    }

    #[test]
    fn update_node_emits_match_set_remove() {
        let patch = PropPatch::new()
            .set("a", PropValue::Int(1))
            .unset("b");
        let s = emit_update_node(NodeId(42), &patch).unwrap();
        assert!(s.cypher.contains("MATCH (n_upd_42) WHERE id(n_upd_42) = $id_42"));
        assert!(s.cypher.contains("SET n_upd_42.`a` = $set_42_a"));
        assert!(s.cypher.contains("REMOVE n_upd_42.`b`"));
        assert_eq!(s.params.get("id_42"), Some(&PropValue::Int(42)));
    }

    #[test]
    fn delete_node_detach() {
        let s = emit_delete_node(NodeId(5), CascadeRule::Detach).unwrap();
        assert!(s.cypher.contains("MATCH (n_del_5) WHERE id(n_del_5) = $id_5 DETACH DELETE n_del_5"));
    }

    #[test]
    fn delete_node_strict() {
        let s = emit_delete_node(NodeId(5), CascadeRule::Strict).unwrap();
        assert!(s.cypher.contains("MATCH (n_del_5) WHERE id(n_del_5) = $id_5 DELETE n_del_5"));
        assert!(!s.cypher.contains("DETACH"));
    }
}
