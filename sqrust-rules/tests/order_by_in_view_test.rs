use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::order_by_in_view::OrderByInView;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    OrderByInView.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(OrderByInView.name(), "Lint/OrderByInView");
}

#[test]
fn create_view_with_order_by_one_violation() {
    let sql = "CREATE VIEW v AS SELECT a FROM t ORDER BY a";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_view_no_order_by_no_violation() {
    let sql = "CREATE VIEW v AS SELECT a FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_view_order_by_with_limit_no_violation() {
    let sql = "CREATE VIEW v AS SELECT a FROM t ORDER BY a LIMIT 10";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_view_order_by_with_fetch_no_violation() {
    let sql = "CREATE VIEW v AS SELECT a FROM t ORDER BY a FETCH FIRST 10 ROWS ONLY";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn plain_select_order_by_no_violation() {
    let sql = "SELECT a FROM t ORDER BY a";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_view_with_where_no_order_by_no_violation() {
    let sql = "CREATE VIEW v AS SELECT a FROM t WHERE x > 0";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_view_multiple_order_by_columns_one_violation() {
    let sql = "CREATE VIEW v AS SELECT a, b FROM t ORDER BY a, b";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "CREATE VIEW @@@###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = OrderByInView.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn create_or_replace_view_with_order_by_one_violation() {
    let sql = "CREATE OR REPLACE VIEW v AS SELECT a FROM t ORDER BY a";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_materialized_view_with_order_by_if_parseable_one_violation() {
    let sql = "CREATE MATERIALIZED VIEW v AS SELECT a FROM t ORDER BY a";
    let ctx = FileContext::from_source(sql, "test.sql");
    if ctx.parse_errors.is_empty() {
        let diags = OrderByInView.check(&ctx);
        assert_eq!(diags.len(), 1);
    }
    // If dialect rejects MATERIALIZED VIEW, parse fails → 0 violations acceptable.
}

#[test]
fn multiple_create_view_with_order_by_multiple_violations() {
    let sql = "CREATE VIEW v1 AS SELECT a FROM t1 ORDER BY a;\nCREATE VIEW v2 AS SELECT b FROM t2 ORDER BY b";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn create_table_as_select_order_by_no_violation() {
    let sql = "CREATE TABLE t AS SELECT a FROM s ORDER BY a";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_contains_order_by_or_view() {
    let sql = "CREATE VIEW v AS SELECT a FROM t ORDER BY a";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("order by") || msg.contains("view"),
        "expected message to mention 'ORDER BY' or 'view', got: {}",
        diags[0].message
    );
}

#[test]
fn diagnostic_rule_name_is_correct() {
    let sql = "CREATE VIEW v AS SELECT a FROM t ORDER BY a";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Lint/OrderByInView");
}

#[test]
fn line_col_nonzero() {
    let sql = "CREATE VIEW v AS SELECT a FROM t ORDER BY a";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
