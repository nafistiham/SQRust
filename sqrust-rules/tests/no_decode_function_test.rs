use sqrust_core::FileContext;
use sqrust_rules::convention::no_decode_function::NoDecodeFunction;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoDecodeFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoDecodeFunction.name(), "Convention/NoDecodeFunction");
}

#[test]
fn decode_basic_violation() {
    let diags = check("SELECT DECODE(col, 1, 'one', 'other') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn decode_lowercase_violation() {
    let diags = check("SELECT decode(col, 1, 'one', 'other') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn decode_mixed_case_violation() {
    let diags = check("SELECT Decode(col, 1, 'one', 'other') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_when_no_violation() {
    let diags = check("SELECT CASE WHEN col = 1 THEN 'one' ELSE 'other' END FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_decode_call_no_violation() {
    let diags = check("SELECT col, 1 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn decode_in_string_no_violation() {
    let diags = check("SELECT 'DECODE(col, 1, 2)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn decode_in_comment_no_violation() {
    let diags = check("-- DECODE(col, 1, 2)\nSELECT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn decode_in_where_clause_violation() {
    let diags = check("SELECT col FROM t WHERE DECODE(status, 1, 'active', 'inactive') = 'active'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_decode_calls_two_violations() {
    let diags = check("SELECT DECODE(a, 1, 'x', 'y'), DECODE(b, 2, 'p', 'q') FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn decode_in_cte_violation() {
    let diags = check("WITH c AS (SELECT DECODE(x, 1, 'a', 'b') AS v FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn decode_in_subquery_violation() {
    let diags = check("SELECT a FROM (SELECT DECODE(b, 0, 'no', 'yes') AS v FROM t) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_decode_and_case_when() {
    let diags = check("SELECT DECODE(col, 1, 'one', 'other') FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("DECODE"),
        "message should contain 'DECODE', got: {msg}"
    );
    assert!(
        upper.contains("CASE WHEN") || upper.contains("CASE"),
        "message should mention CASE WHEN, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT DECODE(col, 1, 'one', 'other') FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn decode_column_name_no_violation() {
    // DECODE used as a column identifier (no paren after) should not be flagged
    let diags = check("SELECT decode_value FROM t");
    assert!(diags.is_empty());
}
