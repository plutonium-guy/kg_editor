//! DDL emission (CREATE CONSTRAINT / INDEX).

use crate::cypher::statement::{sanitize_ident, IdentError, Statement};
use crate::uow::{IndexSpec, NodeConstraint};

pub fn emit_constraint(c: &NodeConstraint) -> Result<Statement, IdentError> {
    match c {
        NodeConstraint::Unique { label, props } => {
            sanitize_ident(label)?;
            let cols = props.iter()
                .map(|p| { sanitize_ident(p).map(|s| format!("n.`{s}`")) })
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            Ok(Statement::new(
                format!(
                    "CREATE CONSTRAINT IF NOT EXISTS FOR (n:`{label}`) REQUIRE ({cols}) IS UNIQUE"
                ),
                std::iter::empty::<(String, _)>(),
            ))
        }
        NodeConstraint::Exists { label, prop } => {
            sanitize_ident(label)?;
            sanitize_ident(prop)?;
            Ok(Statement::new(
                format!(
                    "CREATE CONSTRAINT IF NOT EXISTS FOR (n:`{label}`) REQUIRE n.`{prop}` IS NOT NULL"
                ),
                std::iter::empty::<(String, _)>(),
            ))
        }
    }
}

pub fn emit_index(ix: &IndexSpec) -> Result<Statement, IdentError> {
    sanitize_ident(&ix.label)?;
    let cols = ix.props.iter()
        .map(|p| sanitize_ident(p).map(|s| format!("n.`{s}`")))
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    Ok(Statement::new(
        format!("CREATE INDEX IF NOT EXISTS FOR (n:`{}`) ON ({})", ix.label, cols),
        std::iter::empty::<(String, _)>(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uow::{IndexSpec, NodeConstraint};

    #[test]
    fn emit_unique_constraint() {
        let s = emit_constraint(&NodeConstraint::Unique {
            label: "Person".into(),
            props: vec!["name".into(), "email".into()],
        }).unwrap();
        assert!(s.cypher.starts_with("CREATE CONSTRAINT IF NOT EXISTS"));
        assert!(s.cypher.contains("(n:`Person`)"));
        assert!(s.cypher.contains("n.`name`, n.`email`"));
        assert!(s.cypher.contains("IS UNIQUE"));
    }

    #[test]
    fn emit_exists_constraint() {
        let s = emit_constraint(&NodeConstraint::Exists {
            label: "Person".into(), prop: "name".into(),
        }).unwrap();
        assert!(s.cypher.contains("IS NOT NULL"));
    }

    #[test]
    fn emit_index_test() {
        let s = emit_index(&IndexSpec { label: "Person".into(), props: vec!["name".into()] }).unwrap();
        assert!(s.cypher.starts_with("CREATE INDEX IF NOT EXISTS"));
        assert!(s.cypher.contains("(n:`Person`)"));
    }

    #[test]
    fn rejects_label_with_backtick() {
        let r = emit_index(&IndexSpec { label: "Bad`".into(), props: vec!["x".into()] });
        assert!(r.is_err());
    }
}
