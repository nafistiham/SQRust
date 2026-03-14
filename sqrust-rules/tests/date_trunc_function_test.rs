use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::date_trunc_function::DateTruncFunction;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DateTruncFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DateTruncFunction.name(), "Ambiguous/DateTruncFunction");
}

#[test]
fn date_trunc_violation() {
    let diags = check("SELECT DATE_TRUNC('month', created_at) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/DateTruncFunction");
}

#[test]
fn date_format_violation() {
    let diags = check("SELECT DATE_FORMAT(created_at, '%Y-%m') FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/DateTruncFunction");
}

#[test]
fn trunc_violation() {
    let diags = check("SELECT TRUNC(created_at, 'MM') FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/DateTruncFunction");
}

#[test]
fn date_trunc_case_insensitive() {
    let diags = check("SELECT date_trunc('day', ts) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn date_format_case_insensitive() {
    let diags = check("SELECT date_format(ts, '%Y') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn trunc_case_insensitive() {
    let diags = check("SELECT trunc(ts, 'DD') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_functions_multiple_violations() {
    let sql = "SELECT DATE_TRUNC('month', a), DATE_FORMAT(b, '%Y'), TRUNC(c, 'MM') FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 3);
}

#[test]
fn date_trunc_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE DATE_TRUNC('year', ts) = '2024-01-01'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn date_trunc_in_string_no_violation() {
    let diags = check("SELECT 'DATE_TRUNC example' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn date_trunc_in_comment_no_violation() {
    let diags = check("-- DATE_TRUNC('month', ts)\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn trunc_in_block_comment_no_violation() {
    let diags = check("/* TRUNC(ts) */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn date_trunc_message_content() {
    let diags = check("SELECT DATE_TRUNC('month', ts) FROM t");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_uppercase();
    assert!(
        msg.contains("POSTGRESQL") || msg.contains("DUCKDB"),
        "message should mention PostgreSQL or DuckDB, got: {}",
        diags[0].message
    );
}

#[test]
fn date_format_message_content() {
    let diags = check("SELECT DATE_FORMAT(ts, '%Y') FROM t");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_uppercase();
    assert!(
        msg.contains("MYSQL"),
        "message should mention MySQL, got: {}",
        diags[0].message
    );
}

#[test]
fn trunc_message_content() {
    let diags = check("SELECT TRUNC(ts, 'DD') FROM t");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_uppercase();
    assert!(
        msg.contains("ORACLE") || msg.contains("POSTGRESQL"),
        "message should mention Oracle or PostgreSQL, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT DATE_TRUNC('month', ts) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn truncate_prefix_not_flagged_as_trunc() {
    // TRUNCATE TABLE should not be flagged (TRUNCATE is not followed by '(')
    let diags = check("TRUNCATE TABLE t");
    assert!(diags.is_empty());
}

#[test]
fn two_date_trunc_calls() {
    let diags = check("SELECT DATE_TRUNC('month', a), DATE_TRUNC('year', b) FROM t");
    assert_eq!(diags.len(), 2);
}
