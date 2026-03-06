use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::window_function_without_partition::WindowFunctionWithoutPartition;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    WindowFunctionWithoutPartition.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        WindowFunctionWithoutPartition.name(),
        "Ambiguous/WindowFunctionWithoutPartition"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT OVER BROKEN (((");
    assert!(diags.is_empty());
}

// ── ORDER BY with no PARTITION BY → violation ─────────────────────────────────

#[test]
fn window_with_order_by_no_partition_violation() {
    let diags = check("SELECT ROW_NUMBER() OVER (ORDER BY id) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── PARTITION BY and ORDER BY → no violation ─────────────────────────────────

#[test]
fn window_with_partition_and_order_no_violation() {
    let diags = check("SELECT ROW_NUMBER() OVER (PARTITION BY dept ORDER BY id) FROM t");
    assert!(diags.is_empty());
}

// ── Empty OVER () — no ORDER BY, no PARTITION BY → no violation ───────────────

#[test]
fn window_with_empty_over_no_violation() {
    let diags = check("SELECT SUM(x) OVER () FROM t");
    assert!(diags.is_empty());
}

// ── Only PARTITION BY, no ORDER BY → no violation ────────────────────────────

#[test]
fn window_with_only_partition_no_violation() {
    let diags = check("SELECT SUM(x) OVER (PARTITION BY dept) FROM t");
    assert!(diags.is_empty());
}

// ── No window function at all → no violation ─────────────────────────────────

#[test]
fn no_window_function_no_violation() {
    let diags = check("SELECT SUM(x) FROM t GROUP BY dept");
    assert!(diags.is_empty());
}

// ── RANK() without partition → violation ─────────────────────────────────────

#[test]
fn rank_without_partition_violation() {
    let diags = check("SELECT RANK() OVER (ORDER BY salary DESC) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── Two window functions both missing partition → 2 violations ────────────────

#[test]
fn two_window_functions_both_missing_partition_two_violations() {
    let diags = check(
        "SELECT ROW_NUMBER() OVER (ORDER BY id), RANK() OVER (ORDER BY salary DESC) FROM t",
    );
    assert_eq!(diags.len(), 2);
}

// ── LAG() without partition → violation ──────────────────────────────────────

#[test]
fn lag_without_partition_violation() {
    let diags = check("SELECT LAG(salary) OVER (ORDER BY id) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── Message contains useful text ─────────────────────────────────────────────

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT ROW_NUMBER() OVER (ORDER BY id) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("PARTITION BY"),
        "message should mention PARTITION BY; got: {}",
        diags[0].message
    );
    assert!(
        diags[0].message.contains("ORDER BY"),
        "message should mention ORDER BY; got: {}",
        diags[0].message
    );
}

// ── Line and col are >= 1 ─────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT ROW_NUMBER() OVER (ORDER BY id) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── NTILE() without partition → violation ────────────────────────────────────

#[test]
fn ntile_without_partition_violation() {
    let diags = check("SELECT NTILE(4) OVER (ORDER BY salary) FROM t");
    assert_eq!(diags.len(), 1);
}
