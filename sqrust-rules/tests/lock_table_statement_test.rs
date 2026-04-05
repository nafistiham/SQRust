use sqrust_core::{FileContext, Rule};
use sqrust_rules::lint::lock_table_statement::LockTableStatement;

fn check(sql: &str) -> Vec<String> {
    let ctx = FileContext::from_source(sql, "test.sql");
    LockTableStatement
        .check(&ctx)
        .into_iter()
        .map(|d| format!("{}:{} {}", d.line, d.col, d.message))
        .collect()
}

fn violation_count(sql: &str) -> usize {
    let ctx = FileContext::from_source(sql, "test.sql");
    LockTableStatement.check(&ctx).len()
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(LockTableStatement.name(), "Lint/LockTableStatement");
}

#[test]
fn lock_table_simple_violation() {
    let results = check("LOCK TABLE orders");
    assert_eq!(results.len(), 1);
    assert!(results[0].contains("LOCK TABLE acquires an exclusive lock"));
}

#[test]
fn lock_tables_mysql_violation() {
    let results = check("LOCK TABLES orders READ");
    assert_eq!(results.len(), 1);
    assert!(results[0].contains("LOCK TABLE acquires an exclusive lock"));
}

#[test]
fn lock_table_case_insensitive() {
    let results = check("lock table orders");
    assert_eq!(results.len(), 1);
}

#[test]
fn lock_table_in_string_no_violation() {
    let results = check("SELECT 'lock table' FROM t");
    assert_eq!(results.len(), 0);
}

#[test]
fn lock_table_in_comment_no_violation() {
    let results = check("-- lock table orders\nSELECT 1");
    assert_eq!(results.len(), 0);
}

#[test]
fn no_lock_no_violation() {
    let results = check("SELECT * FROM orders");
    assert_eq!(results.len(), 0);
}

#[test]
fn lock_keyword_without_table_no_violation() {
    // GET_LOCK has no TABLE keyword after LOCK word boundary
    let results = check("SELECT GET_LOCK('name', 10)");
    assert_eq!(results.len(), 0);
}

#[test]
fn multiple_lock_tables_two_violations() {
    let sql = "LOCK TABLE orders;\nLOCK TABLE customers;";
    assert_eq!(violation_count(sql), 2);
}

#[test]
fn lock_table_uppercase_violation() {
    let results = check("LOCK TABLE inventory");
    assert_eq!(results.len(), 1);
}

#[test]
fn lock_table_mixed_case_violation() {
    let results = check("Lock Table products");
    assert_eq!(results.len(), 1);
}

#[test]
fn select_lock_in_name_no_violation() {
    // "lock_count" contains "lock" but is not a standalone keyword
    let results = check("SELECT lock_count FROM t");
    assert_eq!(results.len(), 0);
}

#[test]
fn empty_file_no_violation() {
    let results = check("");
    assert_eq!(results.len(), 0);
}
