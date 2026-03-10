use sqrust_core::{FileContext, Rule};
use sqrust_rules::structure::wildcard_in_union::WildcardInUnion;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    WildcardInUnion.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(WildcardInUnion.name(), "Structure/WildcardInUnion");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn no_union_no_violation() {
    assert!(check("SELECT * FROM t").is_empty());
}

#[test]
fn union_explicit_columns_no_violation() {
    assert!(check("SELECT id, name FROM t UNION ALL SELECT id, name FROM u").is_empty());
}

#[test]
fn wildcard_in_first_union_branch_flagged() {
    let d = check("SELECT * FROM t UNION ALL SELECT id, name FROM u");
    assert_eq!(d.len(), 1);
}

#[test]
fn wildcard_in_second_union_branch_flagged() {
    let d = check("SELECT id, name FROM t UNION ALL SELECT * FROM u");
    assert_eq!(d.len(), 1);
}

#[test]
fn wildcard_in_both_union_branches_flagged() {
    let d = check("SELECT * FROM t UNION ALL SELECT * FROM u");
    assert_eq!(d.len(), 2);
}

#[test]
fn intersect_with_wildcard_flagged() {
    let d = check("SELECT * FROM t INTERSECT SELECT id FROM u");
    assert_eq!(d.len(), 1);
}

#[test]
fn except_with_wildcard_flagged() {
    let d = check("SELECT * FROM t EXCEPT SELECT id FROM u");
    assert_eq!(d.len(), 1);
}

#[test]
fn message_mentions_wildcard_or_union() {
    let d = check("SELECT * FROM t UNION ALL SELECT id FROM u");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("wildcard") || msg.contains("*") || msg.contains("union") || msg.contains("select *"),
        "expected message to mention wildcard or union, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT * FROM t UNION ALL SELECT id FROM u");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Structure/WildcardInUnion");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT * FROM t UNION ALL SELECT id FROM u");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn qualified_wildcard_in_union_flagged() {
    // t.* is also a wildcard
    let d = check("SELECT t.* FROM t UNION ALL SELECT id FROM u");
    assert_eq!(d.len(), 1);
}
