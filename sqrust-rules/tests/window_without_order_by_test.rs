use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::window_without_order_by::WindowWithoutOrderBy;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    WindowWithoutOrderBy.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(WindowWithoutOrderBy.name(), "WindowWithoutOrderBy");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT OVER BROKEN (((");
    assert!(diags.is_empty());
}

// ── ROW_NUMBER() OVER () — no frame, no order → 0 violations ─────────────────

#[test]
fn row_number_over_empty_no_violation() {
    let diags = check("SELECT ROW_NUMBER() OVER () FROM t");
    assert!(diags.is_empty());
}

// ── ROW_NUMBER() OVER (ORDER BY id) → 0 violations ───────────────────────────

#[test]
fn row_number_over_order_by_no_violation() {
    let diags = check("SELECT ROW_NUMBER() OVER (ORDER BY id) FROM t");
    assert!(diags.is_empty());
}

// ── SUM with ORDER BY and frame → 0 violations ────────────────────────────────

#[test]
fn sum_with_order_by_and_frame_no_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t",
    );
    assert!(diags.is_empty());
}

// ── SUM with frame but no ORDER BY → 1 violation ─────────────────────────────

#[test]
fn sum_with_frame_no_order_by_one_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t",
    );
    assert_eq!(diags.len(), 1);
}

// ── SUM with PARTITION BY and frame but no ORDER BY → 1 violation ────────────

#[test]
fn sum_partition_by_frame_no_order_by_one_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY cat ROWS BETWEEN 1 PRECEDING AND CURRENT ROW) FROM t",
    );
    assert_eq!(diags.len(), 1);
}

// ── SUM with PARTITION BY, ORDER BY, and frame → 0 violations ────────────────

#[test]
fn sum_partition_order_by_frame_no_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY cat ORDER BY id ROWS BETWEEN 1 PRECEDING AND CURRENT ROW) FROM t",
    );
    assert!(diags.is_empty());
}

// ── RANGE BETWEEN without ORDER BY → 1 violation ─────────────────────────────

#[test]
fn sum_range_frame_no_order_by_one_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t",
    );
    assert_eq!(diags.len(), 1);
}

// ── SELECT without window function → 0 violations ────────────────────────────

#[test]
fn select_without_window_function_no_violation() {
    let diags = check("SELECT id, name FROM t WHERE id > 0");
    assert!(diags.is_empty());
}

// ── Window function with no frame spec → 0 violations ────────────────────────

#[test]
fn window_function_no_frame_spec_no_violation() {
    let diags = check("SELECT ROW_NUMBER() OVER (PARTITION BY cat ORDER BY id) FROM t");
    assert!(diags.is_empty());
}

// ── Message format is correct ─────────────────────────────────────────────────

#[test]
fn message_format_correct() {
    let diags = check(
        "SELECT SUM(val) OVER (ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t",
    );
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0]
            .message
            .contains("frame specification"),
        "message should mention 'frame specification'"
    );
    assert!(
        diags[0].message.contains("ORDER BY"),
        "message should mention 'ORDER BY'"
    );
}

// ── Line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_is_nonzero() {
    let diags = check(
        "SELECT SUM(val) OVER (ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── Multiple violations in one query ─────────────────────────────────────────

#[test]
fn multiple_violations_in_one_query() {
    let diags = check(
        "SELECT \
            SUM(a) OVER (ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW), \
            AVG(b) OVER (RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) \
         FROM t",
    );
    assert_eq!(diags.len(), 2);
}
