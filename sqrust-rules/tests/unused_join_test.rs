use sqrust_core::{FileContext, Rule};
use sqrust_rules::structure::unused_join::UnusedJoin;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    UnusedJoin.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnusedJoin.name(), "Structure/UnusedJoin");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn no_join_no_violation() {
    assert!(check("SELECT id FROM orders").is_empty());
}

#[test]
fn join_used_in_select_no_violation() {
    assert!(check("SELECT a.id, b.name FROM orders AS a JOIN customers AS b ON a.cid = b.id").is_empty());
}

#[test]
fn join_used_in_where_no_violation() {
    assert!(check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id WHERE b.active = 1").is_empty());
}

#[test]
fn unused_join_flagged() {
    let d = check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_unused_joins_flagged() {
    let d = check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id JOIN products AS p ON a.pid = p.id");
    assert_eq!(d.len(), 2);
}

#[test]
fn join_used_in_order_by_no_violation() {
    assert!(check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id ORDER BY b.name").is_empty());
}

#[test]
fn message_mentions_join_or_table() {
    let d = check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id");
    assert!(d[0].message.to_lowercase().contains("join") || d[0].message.to_lowercase().contains("b"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id");
    assert_eq!(d[0].rule, "Structure/UnusedJoin");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn join_used_in_having_no_violation() {
    assert!(check("SELECT a.id, COUNT(*) FROM orders AS a JOIN customers AS b ON a.cid = b.id GROUP BY a.id HAVING MAX(b.score) > 5").is_empty());
}

#[test]
fn join_without_alias_used_no_violation() {
    assert!(check("SELECT orders.id, customers.name FROM orders JOIN customers ON orders.cid = customers.id").is_empty());
}
