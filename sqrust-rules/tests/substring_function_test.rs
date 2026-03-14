use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::substring_function::SubstringFunction;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SubstringFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SubstringFunction.name(), "Ambiguous/SubstringFunction");
}

#[test]
fn substr_violation() {
    let diags = check("SELECT SUBSTR(col, 1, 3) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/SubstringFunction");
}

#[test]
fn mid_violation() {
    let diags = check("SELECT MID(col, 1, 3) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/SubstringFunction");
}

#[test]
fn substring_no_violation() {
    let diags = check("SELECT SUBSTRING(col, 1, 3) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn substr_case_insensitive() {
    let diags = check("SELECT substr(col, 1, 3) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn mid_case_insensitive() {
    let diags = check("SELECT mid(col, 1, 3) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn substr_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE SUBSTR(name, 1, 2) = 'AB'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn mid_in_cte_violation() {
    let diags = check("WITH c AS (SELECT MID(name, 1, 3) AS short FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_substr_multiple_violations() {
    let diags = check("SELECT SUBSTR(a, 1, 2), SUBSTR(b, 3, 4) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn substr_message_content() {
    let diags = check("SELECT SUBSTR(col, 1, 3) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("SUBSTRING"),
        "message should mention SUBSTRING, got: {msg}"
    );
}

#[test]
fn mid_message_content() {
    let diags = check("SELECT MID(col, 1, 3) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("MYSQL"),
        "message should mention MySQL, got: {msg}"
    );
}

#[test]
fn parse_error_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT SUBSTR(col, 1, 3) FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn substr_and_mid_two_violations() {
    let diags = check("SELECT SUBSTR(a, 1, 2), MID(b, 3, 4) FROM t");
    assert_eq!(diags.len(), 2);
}
