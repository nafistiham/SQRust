use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::integer_division::IntegerDivision;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    IntegerDivision.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(IntegerDivision.name(), "Ambiguous/IntegerDivision");
}

#[test]
fn integer_literal_division_one_violation() {
    let diags = check("SELECT 1/2 FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/IntegerDivision");
}

#[test]
fn float_left_no_violation() {
    // 1.0 / 2 — left side has decimal point, not integer literal
    let diags = check("SELECT 1.0/2 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn float_right_no_violation() {
    // 1 / 2.0 — right side has decimal point, not integer literal
    let diags = check("SELECT 1/2.0 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn five_divided_by_three_one_violation() {
    let diags = check("SELECT 5/3 FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn column_refs_no_violation() {
    let diags = check("SELECT a/b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn column_and_integer_no_violation() {
    // Only flag when BOTH sides are integer literals
    let diags = check("SELECT a/2 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn cast_expression_no_violation() {
    let diags = check("SELECT CAST(1 AS FLOAT)/2 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn integer_division_in_where_one_violation() {
    let diags = check("SELECT * FROM t WHERE 1/2 > 0");
    assert_eq!(diags.len(), 1);
}

#[test]
fn integer_division_in_cte_one_violation() {
    let sql = "WITH cte AS (SELECT 5/3 AS ratio FROM t) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn integer_division_in_subquery_one_violation() {
    let sql = "SELECT * FROM (SELECT 1/2 AS half FROM t) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = IntegerDivision.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn ten_divided_by_five_one_violation() {
    let diags = check("SELECT 10/5 FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_values() {
    let diags = check("SELECT 1/2 FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("1") && diags[0].message.contains("2"),
        "Expected message to contain the values 1 and 2, got: {}",
        diags[0].message
    );
}

#[test]
fn multiple_divisions_multiple_violations() {
    let diags = check("SELECT 1/2, 3/4 FROM t");
    assert_eq!(diags.len(), 2);
}
