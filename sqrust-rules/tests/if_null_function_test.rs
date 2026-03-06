use sqrust_core::FileContext;
use sqrust_rules::convention::if_null_function::IfNullFunction;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    IfNullFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(IfNullFunction.name(), "Convention/IfNullFunction");
}

#[test]
fn parse_error_returns_no_violations() {
    // IfNullFunction is AST-based; a parse failure returns empty.
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn ifnull_one_violation() {
    let diags = check("SELECT IFNULL(name, 'unknown') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nvl_one_violation() {
    let diags = check("SELECT NVL(name, 'unknown') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nvl2_one_violation() {
    let diags = check("SELECT NVL2(name, 'has value', 'null value') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn isnull_two_args_one_violation() {
    let diags = check("SELECT ISNULL(name, 'unknown') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn coalesce_no_violation() {
    let diags = check("SELECT COALESCE(name, 'unknown') FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_null_function_no_violation() {
    let diags = check("SELECT UPPER(name) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn ifnull_case_insensitive_violation() {
    let diags = check("SELECT ifnull(name, 'x') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_function_name() {
    let diags = check("SELECT IFNULL(name, 'x') FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    assert!(
        msg.to_uppercase().contains("IFNULL"),
        "message should contain the function name IFNULL, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT IFNULL(col, 0) FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn two_null_functions_two_violations() {
    let diags = check("SELECT IFNULL(a, ''), NVL(b, '') FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn null_function_in_where_violation() {
    let diags = check("SELECT id FROM t WHERE IFNULL(col, 0) > 5");
    assert_eq!(diags.len(), 1);
}
