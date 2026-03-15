use sqrust_core::FileContext;
use sqrust_rules::convention::no_nvl2::NoNvl2;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoNvl2.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoNvl2.name(), "Convention/NoNvl2");
}

#[test]
fn nvl2_basic_violation() {
    let diags = check("SELECT NVL2(col, 'y', 'n') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nvl2_lowercase_violation() {
    let diags = check("SELECT nvl2(col, 'y', 'n') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nvl2_mixed_case_violation() {
    let diags = check("SELECT Nvl2(col, 'y', 'n') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nvl2_in_string_no_violation() {
    let diags = check("SELECT 'NVL2(col, 1, 0)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nvl2_in_comment_no_violation() {
    let diags = check("-- NVL2(col, 1, 0)\nSELECT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nvl_no_violation() {
    // NVL is a different function — should NOT be flagged by this rule
    let diags = check("SELECT NVL(col, 0) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn coalesce_no_violation() {
    let diags = check("SELECT COALESCE(col, 0) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn two_nvl2_calls_two_violations() {
    let diags = check("SELECT NVL2(a, 1, 0), NVL2(b, 'y', 'n') FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn nvl2_in_where_clause_violation() {
    let diags = check("SELECT col FROM t WHERE NVL2(status, 1, 0) = 1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nvl2_in_cte_violation() {
    let diags = check("WITH c AS (SELECT NVL2(x, 1, 0) AS v FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nvl2_in_subquery_violation() {
    let diags = check("SELECT a FROM (SELECT NVL2(b, 'y', 'n') AS v FROM t) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_case_when_suggestion() {
    let diags = check("SELECT NVL2(col, 'y', 'n') FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("CASE"),
        "message should suggest CASE WHEN, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT NVL2(col, 'y', 'n') FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn nvl2_word_boundary_before_no_violation() {
    // notNVL2( should not match — word char before NVL2
    let diags = check("SELECT notNVL2(col) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nvl2_without_paren_no_violation() {
    // NVL2 as a column alias without a following paren — not a function call
    let diags = check("SELECT col AS nvl2 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nvl2_whitespace_before_paren_violation() {
    // NVL2 followed by whitespace then ( should still be flagged
    let diags = check("SELECT NVL2  (col, 1, 0) FROM t");
    assert_eq!(diags.len(), 1);
}
