use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::implicit_order_direction::ImplicitOrderDirection;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    ImplicitOrderDirection.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ImplicitOrderDirection.name(), "Ambiguous/ImplicitOrderDirection");
}

#[test]
fn order_by_no_direction_violation() {
    let d = check("SELECT * FROM t ORDER BY a");
    assert_eq!(d.len(), 1);
}

#[test]
fn order_by_asc_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY a ASC").is_empty());
}

#[test]
fn order_by_desc_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY a DESC").is_empty());
}

#[test]
fn order_by_multiple_one_implicit_violation() {
    let d = check("SELECT * FROM t ORDER BY a ASC, b");
    assert_eq!(d.len(), 1);
}

#[test]
fn order_by_multiple_both_implicit_two_violations() {
    let d = check("SELECT * FROM t ORDER BY a, b");
    assert_eq!(d.len(), 2);
}

#[test]
fn order_by_multiple_explicit_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY a ASC, b DESC").is_empty());
}

#[test]
fn order_by_in_cte_violation() {
    let sql = "WITH cte AS (SELECT * FROM t ORDER BY a) SELECT * FROM cte";
    let d = check(sql);
    assert_eq!(d.len(), 1);
}

#[test]
fn order_by_in_subquery_violation() {
    let sql = "SELECT * FROM (SELECT * FROM t ORDER BY a) sub";
    let d = check(sql);
    assert_eq!(d.len(), 1);
}

#[test]
fn parse_error_no_violations() {
    // Completely invalid SQL should produce no violations (not crash)
    let d = check("NOT VALID SQL @@@@");
    assert_eq!(d.len(), 0);
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT * FROM t ORDER BY a");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn message_mentions_asc_desc() {
    let d = check("SELECT * FROM t ORDER BY a");
    let msg = d[0].message.to_uppercase();
    assert!(msg.contains("ASC") || msg.contains("DESC"));
}

#[test]
fn no_order_by_no_violation() {
    assert!(check("SELECT * FROM t WHERE id > 1").is_empty());
}

#[test]
fn order_by_position_no_direction_violation() {
    // Position-based ORDER BY without direction is also implicit
    let d = check("SELECT a FROM t ORDER BY 1");
    assert_eq!(d.len(), 1);
}
