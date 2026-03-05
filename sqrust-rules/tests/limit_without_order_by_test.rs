use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::limit_without_order_by::LimitWithoutOrderBy;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    LimitWithoutOrderBy.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(LimitWithoutOrderBy.name(), "Structure/LimitWithoutOrderBy");
}

#[test]
fn limit_without_order_by_one_violation() {
    let diags = check("SELECT col FROM t LIMIT 10");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("non-deterministic"));
}

#[test]
fn order_by_and_limit_no_violation() {
    let diags = check("SELECT col FROM t ORDER BY col LIMIT 10");
    assert!(diags.is_empty());
}

#[test]
fn order_by_desc_and_limit_no_violation() {
    let diags = check("SELECT col FROM t ORDER BY col DESC LIMIT 10");
    assert!(diags.is_empty());
}

#[test]
fn no_limit_no_violation() {
    let diags = check("SELECT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn order_by_without_limit_no_violation() {
    let diags = check("SELECT col FROM t ORDER BY col");
    assert!(diags.is_empty());
}

#[test]
fn limit_zero_is_a_violation() {
    let diags = check("SELECT col FROM t LIMIT 0");
    assert_eq!(diags.len(), 1);
}

#[test]
fn subquery_limit_without_order_by_one_violation() {
    let diags = check("SELECT * FROM (SELECT col FROM t LIMIT 5) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn subquery_with_order_by_and_limit_no_violation() {
    let diags = check("SELECT * FROM (SELECT col FROM t ORDER BY col LIMIT 5) sub");
    assert!(diags.is_empty());
}

#[test]
fn cte_with_limit_without_order_by_one_violation() {
    let diags = check("WITH cte AS (SELECT col FROM t LIMIT 10) SELECT * FROM cte");
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_limit_without_order_by_one_violation() {
    let diags = check("select col from t limit 10");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_queries_both_without_order_by_two_violations() {
    let diags = check("SELECT a FROM t1 LIMIT 5; SELECT b FROM t2 LIMIT 3");
    assert_eq!(diags.len(), 2);
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = LimitWithoutOrderBy.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn subquery_in_where_clause_one_violation() {
    let diags = check("SELECT * FROM t WHERE id IN (SELECT id FROM t2 LIMIT 5)");
    assert_eq!(diags.len(), 1);
}
