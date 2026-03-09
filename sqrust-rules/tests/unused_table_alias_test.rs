use sqrust_core::{FileContext, Rule};
use sqrust_rules::lint::unused_table_alias::UnusedTableAlias;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    UnusedTableAlias.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnusedTableAlias.name(), "Lint/UnusedTableAlias");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn alias_used_as_qualifier_no_violation() {
    assert!(check("SELECT a.id FROM orders AS a").is_empty());
}

#[test]
fn no_alias_no_violation() {
    assert!(check("SELECT id FROM orders").is_empty());
}

#[test]
fn alias_used_in_join_no_violation() {
    assert!(check("SELECT a.id, b.name FROM orders AS a JOIN customers AS b ON a.cid = b.id").is_empty());
}

#[test]
fn unused_alias_flagged() {
    let d = check("SELECT id FROM orders AS o");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_unused_aliases_flagged() {
    let d = check("SELECT id FROM orders AS o JOIN customers AS c ON orders.id = customers.id");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_alias_name() {
    let d = check("SELECT id FROM orders AS o");
    assert!(d[0].message.contains('o') || d[0].message.to_lowercase().contains("alias"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM orders AS o");
    assert_eq!(d[0].rule, "Lint/UnusedTableAlias");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM orders AS o");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn alias_used_in_where_no_violation() {
    assert!(check("SELECT id FROM orders AS o WHERE o.status = 1").is_empty());
}

#[test]
fn alias_used_in_order_by_no_violation() {
    assert!(check("SELECT id FROM orders AS o ORDER BY o.created_at").is_empty());
}

#[test]
fn subquery_alias_used_no_violation() {
    assert!(check("SELECT sub.id FROM (SELECT id FROM t) AS sub WHERE sub.id > 1").is_empty());
}

#[test]
fn subquery_alias_unused_flagged() {
    let d = check("SELECT id FROM (SELECT id FROM t) AS sub");
    assert_eq!(d.len(), 1);
}
