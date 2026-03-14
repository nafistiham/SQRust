use sqrust_core::FileContext;
use sqrust_rules::convention::explicit_column_alias::ExplicitColumnAlias;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ExplicitColumnAlias.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ExplicitColumnAlias.name(), "Convention/ExplicitColumnAlias");
}

#[test]
fn implicit_column_alias_one_violation() {
    let diags = check("SELECT col renamed FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn explicit_column_alias_no_violation() {
    let diags = check("SELECT col AS renamed FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_alias_no_violation() {
    let diags = check("SELECT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn expression_alias_without_as_one_violation() {
    let diags = check("SELECT 1 + 2 result FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn expression_alias_with_as_no_violation() {
    let diags = check("SELECT 1 + 2 AS result FROM t");
    assert!(diags.is_empty());
}

#[test]
fn wildcard_no_violation() {
    let diags = check("SELECT * FROM t");
    assert!(diags.is_empty());
}

#[test]
fn in_cte_one_violation() {
    let diags = check("WITH c AS (SELECT col renamed FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn in_subquery_one_violation() {
    let diags = check("SELECT x FROM (SELECT col renamed FROM t) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_implicit_aliases_two_violations() {
    let diags = check("SELECT a x, b y FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn one_with_as_one_without_one_violation() {
    let diags = check("SELECT a AS x, b y FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn function_result_aliased_without_as_one_violation() {
    let diags = check("SELECT UPPER(col) upper_col FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_alias_name() {
    let diags = check("SELECT col renamed FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    assert!(
        msg.contains("renamed"),
        "message should contain the alias name 'renamed', got: {msg}"
    );
}
