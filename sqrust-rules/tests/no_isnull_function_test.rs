use sqrust_core::FileContext;
use sqrust_rules::convention::no_isnull_function::NoIsnullFunction;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoIsnullFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoIsnullFunction.name(), "Convention/NoIsnullFunction");
}

#[test]
fn isnull_basic_violation() {
    let diags = check("SELECT ISNULL(col, 0) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn isnull_lowercase_violation() {
    let diags = check("SELECT isnull(col, 0) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn isnull_mixed_case_violation() {
    let diags = check("SELECT IsNull(col, 0) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn is_null_predicate_no_violation() {
    let diags = check("SELECT col FROM t WHERE col IS NULL");
    assert!(diags.is_empty());
}

#[test]
fn coalesce_no_violation() {
    let diags = check("SELECT COALESCE(col, 0) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn isnull_in_string_no_violation() {
    let diags = check("SELECT 'ISNULL(col, 0)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn isnull_in_comment_no_violation() {
    let diags = check("-- ISNULL(col, 0)\nSELECT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn isnull_in_where_clause_violation() {
    let diags = check("SELECT col FROM t WHERE ISNULL(status, 0) = 1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_isnull_calls_two_violations() {
    let diags = check("SELECT ISNULL(a, 0), ISNULL(b, '') FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn isnull_in_cte_violation() {
    let diags = check("WITH c AS (SELECT ISNULL(x, 0) AS v FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn isnull_in_subquery_violation() {
    let diags = check("SELECT a FROM (SELECT ISNULL(b, 0) AS v FROM t) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_isnull_and_coalesce() {
    let diags = check("SELECT ISNULL(col, 0) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("ISNULL"),
        "message should contain 'ISNULL', got: {msg}"
    );
    assert!(
        upper.contains("COALESCE"),
        "message should mention COALESCE, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT ISNULL(col, 0) FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn isnull_prefix_column_name_no_violation() {
    // isnull_flag used as a column identifier (no paren after) should not be flagged
    let diags = check("SELECT isnull_flag FROM t");
    assert!(diags.is_empty());
}

#[test]
fn isnull_word_boundary_before_no_violation() {
    // notisnull( should not match because there is a word char before ISNULL
    let diags = check("SELECT notisnull(col) FROM t");
    assert!(diags.is_empty());
}
