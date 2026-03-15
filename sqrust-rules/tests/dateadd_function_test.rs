use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::dateadd_function::DateaddFunction;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DateaddFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DateaddFunction.name(), "Ambiguous/DateaddFunction");
}

#[test]
fn dateadd_uppercase_violation() {
    let diags = check("SELECT DATEADD(day, 1, created_at) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/DateaddFunction");
}

#[test]
fn dateadd_lowercase_violation() {
    let diags = check("SELECT dateadd(day, 1, created_at) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn dateadd_mixedcase_violation() {
    let diags = check("SELECT DateAdd(day, 1, created_at) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn date_add_uppercase_violation() {
    let diags = check("SELECT DATE_ADD(created_at, INTERVAL 1 DAY) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/DateaddFunction");
}

#[test]
fn date_add_lowercase_violation() {
    let diags = check("SELECT date_add(created_at, interval 1 day) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn dateadd_message_mentions_interval() {
    let diags = check("SELECT DATEADD(day, 1, col) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("INTERVAL"),
        "message should mention INTERVAL, got: {}",
        diags[0].message
    );
}

#[test]
fn date_add_message_mentions_interval() {
    let diags = check("SELECT DATE_ADD(col, INTERVAL 1 DAY) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("INTERVAL"),
        "message should mention INTERVAL, got: {}",
        diags[0].message
    );
}

#[test]
fn dateadd_message_mentions_sql_server() {
    let diags = check("SELECT DATEADD(day, 1, col) FROM t");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("sql server"),
        "message should mention SQL Server, got: {}",
        diags[0].message
    );
}

#[test]
fn date_add_message_mentions_mysql() {
    let diags = check("SELECT DATE_ADD(col, INTERVAL 1 DAY) FROM t");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("mysql"),
        "message should mention MySQL, got: {}",
        diags[0].message
    );
}

#[test]
fn dateadd_in_string_no_violation() {
    let diags = check("SELECT 'DATEADD(day, 1, col)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn dateadd_in_line_comment_no_violation() {
    let diags = check("-- DATEADD(day, 1, col)\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn dateadd_in_block_comment_no_violation() {
    let diags = check("/* DATEADD(day, 1, col) */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn date_add_in_string_no_violation() {
    let diags = check("SELECT 'DATE_ADD(col, INTERVAL 1 DAY)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn both_functions_two_violations() {
    let sql = "SELECT DATEADD(day, 1, a), DATE_ADD(b, INTERVAL 1 MONTH) FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn word_boundary_not_flagged() {
    // XDATEADD and DATEADDX should not be flagged
    let diags = check("SELECT XDATEADD(day, 1, col) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT DATEADD(day, 1, col) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn empty_source_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn standard_interval_no_violation() {
    let diags = check("SELECT col + INTERVAL 1 DAY FROM t");
    assert!(diags.is_empty());
}
