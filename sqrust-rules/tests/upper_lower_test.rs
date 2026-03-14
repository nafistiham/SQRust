use sqrust_core::FileContext;
use sqrust_rules::convention::upper_lower::UpperLower;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    UpperLower.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UpperLower.name(), "Convention/UpperLower");
}

#[test]
fn ucase_one_violation() {
    let diags = check("SELECT UCASE(col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn lcase_one_violation() {
    let diags = check("SELECT LCASE(col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn upper_no_violation() {
    let diags = check("SELECT UPPER(col) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn lower_no_violation() {
    let diags = check("SELECT LOWER(col) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn ucase_case_insensitive_violation() {
    let diags = check("SELECT ucase(col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn lcase_case_insensitive_violation() {
    let diags = check("SELECT lcase(col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn ucase_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE UCASE(name) = 'FOO'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn lcase_in_cte_violation() {
    let diags = check("WITH c AS (SELECT LCASE(x) AS l FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn ucase_in_subquery_violation() {
    let diags = check("SELECT a FROM (SELECT UCASE(b) AS u FROM t) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn both_ucase_and_lcase_two_violations() {
    let diags = check("SELECT UCASE(a), LCASE(b) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn ucase_message_mentions_ucase_and_upper() {
    let diags = check("SELECT UCASE(col) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("UCASE"),
        "message should mention UCASE, got: {msg}"
    );
    assert!(
        upper.contains("UPPER"),
        "message should mention UPPER, got: {msg}"
    );
}

#[test]
fn lcase_message_mentions_lcase_and_lower() {
    let diags = check("SELECT LCASE(col) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("LCASE"),
        "message should mention LCASE, got: {msg}"
    );
    assert!(
        upper.contains("LOWER"),
        "message should mention LOWER, got: {msg}"
    );
}
