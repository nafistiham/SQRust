use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::inconsistent_order_by_direction::InconsistentOrderByDirection;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    InconsistentOrderByDirection.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(InconsistentOrderByDirection.name(), "Ambiguous/InconsistentOrderByDirection");
}

#[test]
fn all_explicit_asc_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY a ASC, b ASC").is_empty());
}

#[test]
fn all_explicit_desc_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY a DESC, b DESC").is_empty());
}

#[test]
fn all_implicit_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY a, b, c").is_empty());
}

#[test]
fn single_column_explicit_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY a ASC").is_empty());
}

#[test]
fn single_column_implicit_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY a").is_empty());
}

#[test]
fn mixed_asc_and_implicit_flagged() {
    let d = check("SELECT * FROM t ORDER BY a ASC, b");
    assert_eq!(d.len(), 1);
}

#[test]
fn mixed_desc_and_implicit_flagged() {
    let d = check("SELECT * FROM t ORDER BY a, b DESC");
    assert_eq!(d.len(), 1);
}

#[test]
fn mixed_asc_desc_and_implicit_flagged() {
    let d = check("SELECT * FROM t ORDER BY a ASC, b DESC, c");
    assert_eq!(d.len(), 1);
}

#[test]
fn message_mentions_direction_or_consistent() {
    let d = check("SELECT * FROM t ORDER BY a ASC, b");
    let msg = d[0].message.to_uppercase();
    assert!(msg.contains("ASC") || msg.contains("DESC") || msg.contains("DIRECTION") || msg.contains("CONSISTENT"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT * FROM t ORDER BY a ASC, b");
    assert_eq!(d[0].rule, "Ambiguous/InconsistentOrderByDirection");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT * FROM t ORDER BY a ASC, b");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn asc_in_string_not_counted() {
    assert!(check("SELECT 'ASC' FROM t ORDER BY a ASC, b ASC").is_empty());
}

#[test]
fn no_order_by_no_violation() {
    assert!(check("SELECT * FROM t WHERE id > 1").is_empty());
}
