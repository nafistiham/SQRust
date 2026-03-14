use sqrust_core::FileContext;
use sqrust_rules::convention::len_function::LenFunction;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    LenFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(LenFunction.name(), "Convention/LenFunction");
}

#[test]
fn len_col_one_violation() {
    let diags = check("SELECT LEN(col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn length_col_no_violation() {
    let diags = check("SELECT LENGTH(col) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn len_case_insensitive_violation() {
    let diags = check("SELECT len(col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn len_mixed_case_violation() {
    let diags = check("SELECT Len(col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn len_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE LEN(name) > 5");
    assert_eq!(diags.len(), 1);
}

#[test]
fn len_in_cte_violation() {
    let diags = check("WITH c AS (SELECT LEN(x) AS l FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn len_in_subquery_violation() {
    let diags = check("SELECT a FROM (SELECT LEN(b) AS l FROM t) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nested_len_two_violations() {
    let diags = check("SELECT LEN(LEN(x)) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn multiple_len_calls_correct_count() {
    let diags = check("SELECT LEN(a), LEN(b), LEN(c) FROM t");
    assert_eq!(diags.len(), 3);
}

#[test]
fn strlen_no_violation() {
    // STRLEN is not the flagged function — only exact name LEN
    let diags = check("SELECT STRLEN(col) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_len_and_length() {
    let diags = check("SELECT LEN(col) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("LEN"),
        "message should mention LEN, got: {msg}"
    );
    assert!(
        upper.contains("LENGTH"),
        "message should mention LENGTH, got: {msg}"
    );
}

#[test]
fn diagnostic_rule_name_matches() {
    let diags = check("SELECT LEN(col) FROM t");
    assert!(!diags.is_empty());
    assert_eq!(diags[0].rule, "Convention/LenFunction");
}
