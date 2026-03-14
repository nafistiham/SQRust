use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::convert_function::ConvertFunction;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ConvertFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ConvertFunction.name(), "Ambiguous/ConvertFunction");
}

#[test]
fn convert_violation() {
    let diags = check("SELECT CONVERT(INT, col) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/ConvertFunction");
}

#[test]
fn cast_no_violation() {
    let diags = check("SELECT CAST(col AS INT) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn convert_case_insensitive() {
    let diags = check("SELECT convert(int, col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn convert_mixed_case() {
    let diags = check("SELECT Convert(INT, col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn convert_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE CONVERT(INT, col) > 5");
    assert_eq!(diags.len(), 1);
}

#[test]
fn convert_in_line_comment_no_violation() {
    let diags = check("-- CONVERT(INT, col)\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn convert_in_block_comment_no_violation() {
    let diags = check("/* CONVERT(INT, col) */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn convert_in_string_no_violation() {
    let diags = check("SELECT 'CONVERT(INT, col)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn multiple_convert_calls_multiple_violations() {
    let sql = "SELECT CONVERT(INT, a), CONVERT(VARCHAR, b) FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn convert_message_mentions_cast() {
    let diags = check("SELECT CONVERT(INT, col) FROM t");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_uppercase();
    assert!(
        msg.contains("CAST"),
        "message should mention CAST, got: {}",
        diags[0].message
    );
}

#[test]
fn convert_message_mentions_dialect() {
    let diags = check("SELECT CONVERT(INT, col) FROM t");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_uppercase();
    assert!(
        msg.contains("DIALECT") || msg.contains("SQL SERVER") || msg.contains("MYSQL"),
        "message should mention dialect info, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT CONVERT(INT, col) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn convert_word_boundary_before() {
    // RECONVERT( should not be flagged — word char before CONVERT
    let diags = check("SELECT RECONVERT(col) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn convert_in_cte_violation() {
    let sql = "WITH c AS (SELECT CONVERT(DATE, ts) FROM t) SELECT * FROM c";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn convert_line_reported() {
    let sql = "SELECT 1\nFROM t\nWHERE CONVERT(INT, col) = 5";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}
