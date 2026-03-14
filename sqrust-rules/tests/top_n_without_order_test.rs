use sqrust_core::FileContext;
use sqrust_rules::convention::top_n_without_order::TopNWithoutOrder;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    TopNWithoutOrder.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(TopNWithoutOrder.name(), "Convention/TopNWithoutOrder");
}

#[test]
fn top_without_order_by_violation() {
    let diags = check("SELECT TOP 10 * FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn top_with_order_by_no_violation() {
    let diags = check("SELECT TOP 10 * FROM t ORDER BY id");
    assert!(diags.is_empty());
}

#[test]
fn top_case_insensitive() {
    let diags = check("select top 5 a from t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_top_no_violation() {
    let diags = check("SELECT * FROM t");
    assert!(diags.is_empty());
}

#[test]
fn top_with_order_by_in_cte_no_violation() {
    let diags = check(
        "WITH cte AS (SELECT TOP 5 id FROM t ORDER BY id) SELECT * FROM cte",
    );
    assert!(diags.is_empty());
}

#[test]
fn top_in_string_no_violation() {
    let diags = check("SELECT 'SELECT TOP 10 * FROM t' AS q FROM dual");
    assert!(diags.is_empty());
}

#[test]
fn top_in_comment_no_violation() {
    let diags = check("-- SELECT TOP 10 * FROM t\nSELECT a FROM t");
    assert!(diags.is_empty());
}

#[test]
fn top_percent_without_order_violation() {
    let diags = check("SELECT TOP 10 PERCENT * FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn top_percent_with_order_no_violation() {
    let diags = check("SELECT TOP 10 PERCENT * FROM t ORDER BY id");
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_order_by() {
    let diags = check("SELECT TOP 10 * FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("ORDER BY"),
        "message should mention ORDER BY, got: {msg}"
    );
}

#[test]
fn message_mentions_limit() {
    let diags = check("SELECT TOP 10 * FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("LIMIT"),
        "message should mention LIMIT, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT TOP 10 * FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn select_top_1_with_order_no_violation() {
    let diags = check("SELECT TOP 1 * FROM t ORDER BY created_at DESC");
    assert!(diags.is_empty());
}
