use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::window_function_in_where::WindowFunctionInWhere;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    WindowFunctionInWhere.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        WindowFunctionInWhere.name(),
        "Structure/WindowFunctionInWhere"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_no_violations() {
    let diags = check("SELECT FROM FROM WHERE");
    assert!(diags.is_empty());
}

// ── window function in WHERE — 1 violation ───────────────────────────────────

#[test]
fn window_function_in_where_violation() {
    // ROW_NUMBER() OVER (...) = 1 in WHERE clause
    let diags = check(
        "SELECT * FROM t WHERE ROW_NUMBER() OVER (PARTITION BY a ORDER BY b) = 1",
    );
    assert_eq!(diags.len(), 1);
}

// ── window function in SELECT — no violation ─────────────────────────────────

#[test]
fn window_function_in_select_no_violation() {
    let diags = check("SELECT ROW_NUMBER() OVER (ORDER BY a) AS rn FROM t");
    assert!(diags.is_empty());
}

// ── aggregate in WHERE — no violation (different from window) ─────────────────

#[test]
fn aggregate_in_where_no_violation() {
    // Aggregates in WHERE are caught by a different rule; this rule ignores them.
    let diags = check("SELECT id FROM t WHERE id > 1");
    assert!(diags.is_empty());
}

// ── window function in HAVING — no violation ─────────────────────────────────

#[test]
fn window_function_in_having_no_violation() {
    // HAVING is different from WHERE; no violation.
    let diags = check(
        "SELECT a, SUM(b) FROM t GROUP BY a \
         HAVING SUM(b) > ROW_NUMBER() OVER (ORDER BY a)",
    );
    assert!(diags.is_empty());
}

// ── multiple window functions in WHERE — multiple violations ──────────────────

#[test]
fn multiple_window_functions_in_where_multiple_violations() {
    let diags = check(
        "SELECT * FROM t \
         WHERE ROW_NUMBER() OVER (ORDER BY a) = 1 \
           AND RANK() OVER (ORDER BY b) < 5",
    );
    assert_eq!(diags.len(), 2);
}

// ── window function in subquery WHERE — violation ─────────────────────────────

#[test]
fn window_function_in_subquery_where_violation() {
    let diags = check(
        "SELECT x FROM (SELECT * FROM t WHERE ROW_NUMBER() OVER (ORDER BY a) = 1) sub",
    );
    assert_eq!(diags.len(), 1);
}

// ── ROW_NUMBER window function ────────────────────────────────────────────────

#[test]
fn row_number_window_function_violation() {
    let diags = check(
        "SELECT id FROM t WHERE ROW_NUMBER() OVER (ORDER BY id) <= 10",
    );
    assert_eq!(diags.len(), 1);
}

// ── RANK window function ──────────────────────────────────────────────────────

#[test]
fn rank_window_function_violation() {
    let diags = check(
        "SELECT id FROM t WHERE RANK() OVER (PARTITION BY cat ORDER BY score DESC) = 1",
    );
    assert_eq!(diags.len(), 1);
}

// ── SUM OVER (analytic) in WHERE ──────────────────────────────────────────────

#[test]
fn sum_over_partition_in_where_violation() {
    let diags = check(
        "SELECT id FROM t WHERE SUM(amount) OVER (PARTITION BY category) > 1000",
    );
    assert_eq!(diags.len(), 1);
}

// ── message mentions subquery ─────────────────────────────────────────────────

#[test]
fn message_mentions_subquery() {
    let diags = check(
        "SELECT * FROM t WHERE ROW_NUMBER() OVER (ORDER BY a) = 1",
    );
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("subquery") || msg.contains("cte") || msg.contains("window"),
        "expected message to mention subquery/CTE/window, got: {}",
        diags[0].message
    );
}

// ── line and col are nonzero ──────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check(
        "SELECT * FROM t WHERE ROW_NUMBER() OVER (ORDER BY a) = 1",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── window function in CTE WHERE — violation ──────────────────────────────────

#[test]
fn window_function_in_cte_where_violation() {
    let diags = check(
        "WITH ranked AS (\
           SELECT * FROM t \
           WHERE ROW_NUMBER() OVER (PARTITION BY a ORDER BY b) = 1\
         ) SELECT id FROM ranked",
    );
    assert_eq!(diags.len(), 1);
}

// ── plain SELECT, no WHERE — no violation ─────────────────────────────────────

#[test]
fn no_where_clause_no_violation() {
    let diags = check("SELECT ROW_NUMBER() OVER (ORDER BY id) AS rn FROM t");
    assert!(diags.is_empty());
}
