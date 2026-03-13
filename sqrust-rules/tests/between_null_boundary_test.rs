use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::between_null_boundary::BetweenNullBoundary;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    BetweenNullBoundary.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(BetweenNullBoundary.name(), "Ambiguous/BetweenNullBoundary");
}

#[test]
fn between_null_and_value_one_violation() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN NULL AND 10");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/BetweenNullBoundary");
}

#[test]
fn between_value_and_value_no_violation() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN 1 AND 10");
    assert!(diags.is_empty());
}

#[test]
fn between_value_and_null_one_violation() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN 1 AND NULL");
    assert_eq!(diags.len(), 1);
}

#[test]
fn between_null_and_null_one_violation() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN NULL AND NULL");
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_between_null_and_value_one_violation() {
    let diags = check("SELECT * FROM t WHERE col NOT BETWEEN NULL AND 10");
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_between_value_and_value_no_violation() {
    let diags = check("SELECT * FROM t WHERE col NOT BETWEEN 1 AND 10");
    assert!(diags.is_empty());
}

#[test]
fn between_zero_and_hundred_no_violation() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN 0 AND 100");
    assert!(diags.is_empty());
}

#[test]
fn between_null_in_case_expression_one_violation() {
    let diags = check(
        "SELECT CASE WHEN col BETWEEN NULL AND 5 THEN 'y' ELSE 'n' END FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn between_null_in_subquery_one_violation() {
    let diags = check(
        "SELECT a FROM (SELECT * FROM t WHERE col BETWEEN NULL AND 10) sub",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn between_null_in_cte_one_violation() {
    let diags = check(
        "WITH c AS (SELECT * FROM t WHERE col BETWEEN NULL AND 10) SELECT * FROM c",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = BetweenNullBoundary.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn between_column_refs_no_violation() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN a AND b");
    assert!(diags.is_empty());
}

#[test]
fn message_contains_null_or_between() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN NULL AND 10");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("NULL") || msg.contains("BETWEEN"),
        "message was: {msg}"
    );
}
