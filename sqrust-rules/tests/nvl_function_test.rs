use sqrust_core::FileContext;
use sqrust_rules::convention::nvl_function::NvlFunction;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NvlFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NvlFunction.name(), "Convention/NvlFunction");
}

#[test]
fn nvl_basic_violation() {
    let diags = check("SELECT NVL(col, 0) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nvl_lowercase_violation() {
    let diags = check("SELECT nvl(col, 0) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nvl_mixed_case_violation() {
    let diags = check("SELECT Nvl(col, 0) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nvl2_basic_violation() {
    let diags = check("SELECT NVL2(col, 'not null', 'null') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nvl2_lowercase_violation() {
    let diags = check("SELECT nvl2(col, 'x', 'y') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn coalesce_no_violation() {
    let diags = check("SELECT COALESCE(col, 0) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nvl_in_string_no_violation() {
    let diags = check("SELECT 'NVL(col, 0)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nvl_in_comment_no_violation() {
    let diags = check("-- NVL(col, 0)\nSELECT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nvl2_in_string_no_violation() {
    let diags = check("SELECT 'NVL2(col, 1, 0)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nvl_in_where_clause_violation() {
    let diags = check("SELECT col FROM t WHERE NVL(status, 0) = 1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_nvl_calls_two_violations() {
    let diags = check("SELECT NVL(a, 0), NVL(b, 1) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn nvl_and_nvl2_both_violations() {
    let diags = check("SELECT NVL(a, 0), NVL2(b, 'x', 'y') FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn nvl_message_contains_coalesce() {
    let diags = check("SELECT NVL(col, 0) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("COALESCE"),
        "NVL message should suggest COALESCE, got: {msg}"
    );
}

#[test]
fn nvl2_message_contains_case_when() {
    let diags = check("SELECT NVL2(col, 'not null', 'null') FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("CASE WHEN") || upper.contains("CASE"),
        "NVL2 message should suggest CASE WHEN, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT NVL(col, 0) FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
