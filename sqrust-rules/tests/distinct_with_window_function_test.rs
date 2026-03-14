use sqrust_core::FileContext;
use sqrust_rules::ambiguous::distinct_with_window_function::DistinctWithWindowFunction;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    DistinctWithWindowFunction.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        DistinctWithWindowFunction.name(),
        "Ambiguous/DistinctWithWindowFunction"
    );
}

#[test]
fn select_distinct_with_row_number_violation() {
    let diags = check("SELECT DISTINCT a, ROW_NUMBER() OVER (ORDER BY a) AS rn FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_distinct_no_window_fn_no_violation() {
    let diags = check("SELECT DISTINCT a, b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn select_no_distinct_with_window_fn_no_violation() {
    let diags = check("SELECT a, ROW_NUMBER() OVER (ORDER BY a) AS rn FROM t");
    assert!(diags.is_empty());
}

#[test]
fn select_distinct_with_rank_violation() {
    let diags = check("SELECT DISTINCT a, RANK() OVER (PARTITION BY b ORDER BY a) AS r FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_distinct_with_dense_rank_violation() {
    let diags = check("SELECT DISTINCT a, DENSE_RANK() OVER (ORDER BY a) AS dr FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_distinct_with_sum_aggregate_no_violation() {
    // SUM without OVER is a plain aggregate, not a window function
    let diags = check("SELECT DISTINCT a, SUM(b) FROM t GROUP BY a");
    assert!(diags.is_empty());
}

#[test]
fn select_distinct_with_multiple_window_fns_one_violation() {
    // Multiple window functions in the same SELECT should produce exactly 1 diagnostic per SELECT
    let sql = "SELECT DISTINCT a, ROW_NUMBER() OVER (ORDER BY a), RANK() OVER (ORDER BY a) FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_distinct_with_window_fn_in_subquery_violation() {
    let sql = "SELECT * FROM (SELECT DISTINCT a, ROW_NUMBER() OVER (ORDER BY a) AS rn FROM t) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_distinct_with_window_fn_in_cte_violation() {
    let sql = "WITH cte AS (SELECT DISTINCT a, ROW_NUMBER() OVER (ORDER BY a) AS rn FROM t) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn outer_no_distinct_inner_has_distinct_with_window_fn_violation() {
    // Violation is in the inner SELECT DISTINCT
    let sql = "SELECT x FROM (SELECT DISTINCT a, ROW_NUMBER() OVER (ORDER BY a) rn FROM t) s";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_content_is_informative() {
    let diags = check("SELECT DISTINCT a, ROW_NUMBER() OVER (ORDER BY a) AS rn FROM t");
    let msg = &diags[0].message;
    assert!(
        msg.contains("DISTINCT") || msg.contains("window"),
        "message was: {msg}"
    );
}

#[test]
fn line_col_are_nonzero() {
    let diags = check("SELECT DISTINCT a, ROW_NUMBER() OVER (ORDER BY a) AS rn FROM t");
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn select_distinct_with_lag_window_fn_violation() {
    let diags = check("SELECT DISTINCT a, LAG(a) OVER (ORDER BY a) AS prev FROM t");
    assert_eq!(diags.len(), 1);
}
