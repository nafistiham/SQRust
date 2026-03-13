use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::set_op_precedence::SetOpPrecedence;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SetOpPrecedence.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SetOpPrecedence.name(), "Structure/SetOpPrecedence");
}

#[test]
fn union_then_intersect_one_violation() {
    let diags = check("SELECT a FROM t UNION SELECT a FROM s INTERSECT SELECT a FROM u");
    assert_eq!(diags.len(), 1);
}

#[test]
fn only_union_no_violation() {
    let diags = check("SELECT a FROM t UNION SELECT a FROM s");
    assert!(diags.is_empty());
}

#[test]
fn only_intersect_no_violation() {
    let diags = check("SELECT a FROM t INTERSECT SELECT a FROM s");
    assert!(diags.is_empty());
}

#[test]
fn only_except_no_violation() {
    let diags = check("SELECT a FROM t EXCEPT SELECT a FROM s");
    assert!(diags.is_empty());
}

#[test]
fn parenthesized_union_then_intersect_no_violation() {
    let diags = check("(SELECT a FROM t UNION SELECT a FROM s) INTERSECT SELECT a FROM u");
    assert!(diags.is_empty());
}

#[test]
fn union_all_then_intersect_one_violation() {
    let diags = check("SELECT a FROM t UNION ALL SELECT a FROM s INTERSECT SELECT a FROM u");
    assert_eq!(diags.len(), 1);
}

#[test]
fn except_then_intersect_one_violation() {
    let diags = check("SELECT a FROM t EXCEPT SELECT a FROM s INTERSECT SELECT a FROM u");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_unions_no_violation() {
    let diags = check("SELECT a FROM t UNION SELECT a FROM s UNION SELECT a FROM u");
    assert!(diags.is_empty());
}

#[test]
fn multiple_intersects_no_violation() {
    let diags = check("SELECT a FROM t INTERSECT SELECT a FROM s INTERSECT SELECT a FROM u");
    assert!(diags.is_empty());
}

#[test]
fn parse_error_no_violation() {
    let diags = check("NOT VALID SQL @@@");
    assert!(diags.is_empty());
}

#[test]
fn mixed_ops_inside_subquery_one_violation() {
    let diags = check(
        "SELECT x FROM (SELECT a FROM t UNION SELECT a FROM s INTERSECT SELECT a FROM u) sub",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn union_with_inner_intersect_parenthesized_no_violation() {
    let diags =
        check("SELECT a FROM t UNION (SELECT a FROM s INTERSECT SELECT a FROM u)");
    assert!(diags.is_empty());
}

#[test]
fn message_contains_intersect_and_precedence() {
    let diags = check("SELECT a FROM t UNION SELECT a FROM s INTERSECT SELECT a FROM u");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("INTERSECT"));
    assert!(diags[0].message.contains("precedence"));
}
