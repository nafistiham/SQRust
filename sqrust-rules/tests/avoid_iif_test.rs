use sqrust_core::FileContext;
use sqrust_rules::convention::avoid_iif::AvoidIif;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    AvoidIif.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(AvoidIif.name(), "Convention/AvoidIif");
}

#[test]
fn iif_one_violation() {
    let diags = check("SELECT IIF(a > 0, 'yes', 'no') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_when_no_violation() {
    let diags = check("SELECT CASE WHEN a > 0 THEN 'yes' ELSE 'no' END FROM t");
    assert!(diags.is_empty());
}

#[test]
fn iif_case_insensitive_violation() {
    let diags = check("SELECT iif(x, 1, 0) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nested_iif_two_violations() {
    let diags = check("SELECT IIF(a, IIF(b, 1, 2), 3) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn iif_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE IIF(x > 0, 1, 0) = 1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn iif_in_cte_violation() {
    let diags = check("WITH c AS (SELECT IIF(x, 1, 0) AS v FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn iif_in_subquery_violation() {
    let diags = check("SELECT a FROM (SELECT IIF(b, 1, 0) AS v FROM t) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn if_function_no_violation() {
    // IF() is MySQL-specific and not flagged by this rule
    let diags = check("SELECT IF(a > 0, 'yes', 'no') FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nullif_no_violation() {
    let diags = check("SELECT NULLIF(a, b) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn multiple_iif_calls_correct_count() {
    let diags = check("SELECT IIF(a, 1, 0), IIF(b, 2, 3), IIF(c, 4, 5) FROM t");
    assert_eq!(diags.len(), 3);
}

#[test]
fn message_contains_iif_or_case_when() {
    let diags = check("SELECT IIF(a, 1, 0) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("IIF") || upper.contains("CASE WHEN"),
        "message should contain 'IIF' or 'CASE WHEN', got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT IIF(a, 1, 0) FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
