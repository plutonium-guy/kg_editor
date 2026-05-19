use kg_core::cypher::CypherEmitter;
use kg_core::node::NodeId;
use kg_core::uow::{CascadeRule, IndexSpec, NodeConstraint, PropPatch, UnitOfWork};
use kg_core::value::PropValue;

#[test]
fn snapshot_full_uow() {
    let mut uow = UnitOfWork::new();
    uow.ensure_constraint(NodeConstraint::Unique {
        label: "Person".into(),
        props: vec!["name".into()],
    });
    uow.ensure_index(IndexSpec {
        label: "Person".into(),
        props: vec!["age".into()],
    });
    let a = uow.create_node(["Person"], [("name", PropValue::from("Alice"))]);
    let b = uow.create_node(["Person"], [("name", PropValue::from("Bob"))]);
    uow.create_rel(a, b, "KNOWS", [("since", PropValue::Int(2020))]);
    uow.update_node(
        NodeId(42),
        PropPatch::new()
            .set("nick", PropValue::from("X"))
            .unset("dead"),
    );
    uow.delete_node(NodeId(99), CascadeRule::Detach);

    let stmts = CypherEmitter::emit(&uow).unwrap();
    insta::assert_json_snapshot!(stmts);
}
