use sqrust_core::{FileContext, Rule};
use sqrust_rules::lint::duplicate_join::DuplicateJoin;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    DuplicateJoin.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DuplicateJoin.name(), "Lint/DuplicateJoin");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn no_joins_no_violation() {
    assert!(check("SELECT id FROM t WHERE id > 1").is_empty());
}

#[test]
fn two_different_tables_no_violation() {
    assert!(check("SELECT t.id FROM t JOIN u ON t.id = u.t_id").is_empty());
}

#[test]
fn same_table_twice_flagged() {
    let d = check("SELECT a.id FROM orders a JOIN orders b ON a.parent = b.id");
    assert_eq!(d.len(), 1);
}

#[test]
fn same_table_three_times_flagged() {
    let d = check("SELECT a.id FROM t a JOIN t b ON a.p = b.id JOIN t c ON b.p = c.id");
    assert_eq!(d.len(), 1);
}

#[test]
fn main_table_and_join_same_flagged() {
    let d = check("SELECT t.id FROM t JOIN t AS t2 ON t.id = t2.parent_id");
    assert_eq!(d.len(), 1);
}

#[test]
fn schema_qualified_same_table_flagged() {
    let d = check("SELECT a.id FROM schema1.orders a JOIN schema1.orders b ON a.id = b.ref");
    assert_eq!(d.len(), 1);
}

#[test]
fn different_schemas_same_name_no_violation() {
    // schema1.orders and schema2.orders are different tables
    assert!(check("SELECT a.id FROM schema1.orders a JOIN schema2.orders b ON a.id = b.ref").is_empty());
}

#[test]
fn message_mentions_duplicate() {
    let d = check("SELECT a.id FROM t a JOIN t b ON a.id = b.parent");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("duplicate") || msg.contains("joined") || msg.contains("twice") || msg.contains("more than once"),
        "expected message to mention duplicate join, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT a.id FROM t a JOIN t b ON a.id = b.p");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Lint/DuplicateJoin");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT a.id FROM t a JOIN t b ON a.id = b.p");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn subquery_own_joins_checked_independently() {
    // Subquery's own duplicate join should be flagged
    let d = check("SELECT * FROM (SELECT a.id FROM t a JOIN t b ON a.id = b.p) sub");
    assert_eq!(d.len(), 1);
}
