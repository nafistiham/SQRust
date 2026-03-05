use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::order_by_with_offset::OrderByWithOffset;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    OrderByWithOffset.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(OrderByWithOffset.name(), "Convention/OrderByWithOffset");
}

#[test]
fn offset_without_order_by_one_violation() {
    let diags = check("SELECT col FROM t OFFSET 10");
    assert_eq!(diags.len(), 1);
}

#[test]
fn order_by_and_offset_no_violation() {
    let diags = check("SELECT col FROM t ORDER BY col OFFSET 10");
    assert!(diags.is_empty());
}

#[test]
fn order_by_limit_and_offset_no_violation() {
    let diags = check("SELECT col FROM t ORDER BY col LIMIT 20 OFFSET 10");
    assert!(diags.is_empty());
}

#[test]
fn no_offset_no_violation() {
    let diags = check("SELECT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn order_by_without_offset_no_violation() {
    let diags = check("SELECT col FROM t ORDER BY col");
    assert!(diags.is_empty());
}

#[test]
fn lowercase_offset_without_order_by_one_violation() {
    let diags = check("select col from t offset 5");
    assert_eq!(diags.len(), 1);
}

#[test]
fn subquery_offset_without_order_by_one_violation() {
    let diags = check("SELECT * FROM (SELECT col FROM t OFFSET 5) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cte_with_offset_without_order_by_one_violation() {
    let diags = check("WITH cte AS (SELECT col FROM t OFFSET 10) SELECT * FROM cte");
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = OrderByWithOffset.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn message_format_is_correct() {
    let diags = check("SELECT col FROM t OFFSET 10");
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "OFFSET without ORDER BY produces non-deterministic results"
    );
}

#[test]
fn two_queries_one_ok_one_violation() {
    let diags =
        check("SELECT col FROM t ORDER BY col OFFSET 5; SELECT col FROM t2 OFFSET 3");
    assert_eq!(diags.len(), 1);
}

#[test]
fn offset_zero_with_order_by_no_violation() {
    let diags = check("SELECT col FROM t ORDER BY col OFFSET 0");
    assert!(diags.is_empty());
}
