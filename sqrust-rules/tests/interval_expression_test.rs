use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::interval_expression::IntervalExpression;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    IntervalExpression.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(IntervalExpression.name(), "Ambiguous/IntervalExpression");
}

#[test]
fn interval_with_quotes_violation() {
    let diags = check("SELECT * FROM t WHERE d > NOW() - INTERVAL '7' DAY");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/IntervalExpression");
}

#[test]
fn interval_with_number_violation() {
    let diags = check("SELECT * FROM t WHERE d > NOW() - INTERVAL 7 DAY");
    assert_eq!(diags.len(), 1);
}

#[test]
fn interval_case_insensitive() {
    let diags = check("SELECT * FROM t WHERE d > NOW() - interval '1' day");
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_interval_no_violation() {
    let diags = check("SELECT * FROM t WHERE d > '2020-01-01'");
    assert!(diags.is_empty());
}

#[test]
fn interval_in_select_violation() {
    let diags = check("SELECT CURRENT_DATE - INTERVAL '30' DAY FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn interval_in_where_violation() {
    let diags = check("SELECT * FROM orders WHERE created_at > NOW() - INTERVAL '90' DAY");
    assert_eq!(diags.len(), 1);
}

#[test]
fn interval_in_cte_violation() {
    let sql = "WITH cte AS (SELECT * FROM t WHERE ts > NOW() - INTERVAL '1' HOUR) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_intervals_multiple_violations() {
    let sql = "SELECT * FROM t WHERE a > NOW() - INTERVAL '7' DAY AND b < NOW() + INTERVAL 30 DAY";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn interval_in_string_no_violation() {
    let diags = check("SELECT 'INTERVAL 1 DAY example' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn interval_in_comment_no_violation() {
    let diags = check("-- INTERVAL SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn message_content() {
    let diags = check("SELECT * FROM t WHERE d > NOW() - INTERVAL '7' DAY");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("SQL Server") || msg.contains("DATEADD"),
        "Expected message to mention 'SQL Server' or 'DATEADD', got: {}",
        msg
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM t WHERE d > NOW() - INTERVAL '7' DAY");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn interval_column_name_no_violation() {
    // `interval_days` starts with `interval` but has word char after, should not be flagged
    let diags = check("SELECT interval_days FROM t");
    assert!(diags.is_empty());
}

#[test]
fn interval_block_comment_no_violation() {
    let diags = check("/* INTERVAL '1' DAY */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn interval_multiline_violation() {
    let sql = "SELECT *\nFROM t\nWHERE d > NOW() - INTERVAL '7' DAY";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}
