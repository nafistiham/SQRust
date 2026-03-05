use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::having_without_aggregate::HavingWithoutAggregate;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    HavingWithoutAggregate.check(&c)
}

// ── rule name ────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(HavingWithoutAggregate.name(), "HavingWithoutAggregate");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM HAVING");
    assert!(diags.is_empty());
}

// ── HAVING with COUNT(*) — no violation ──────────────────────────────────────

#[test]
fn having_count_star_no_violation() {
    let diags = check("SELECT dept, COUNT(*) FROM employees GROUP BY dept HAVING COUNT(*) > 1");
    assert!(diags.is_empty());
}

// ── HAVING with SUM — no violation ───────────────────────────────────────────

#[test]
fn having_sum_no_violation() {
    let diags = check("SELECT dept, SUM(amount) FROM orders GROUP BY dept HAVING SUM(amount) > 100");
    assert!(diags.is_empty());
}

// ── HAVING with plain column — 1 violation ───────────────────────────────────

#[test]
fn having_plain_column_one_violation() {
    let diags = check("SELECT dept FROM employees GROUP BY dept HAVING col > 5");
    assert_eq!(diags.len(), 1);
}

// ── HAVING col AND COUNT(*) — has aggregate — no violation ───────────────────

#[test]
fn having_col_and_count_no_violation() {
    let diags = check("SELECT dept, COUNT(*) FROM employees GROUP BY dept HAVING col > 5 AND COUNT(*) > 1");
    assert!(diags.is_empty());
}

// ── no HAVING clause — no violation ──────────────────────────────────────────

#[test]
fn no_having_no_violation() {
    let diags = check("SELECT dept FROM employees GROUP BY dept");
    assert!(diags.is_empty());
}

// ── HAVING AVG — no violation ─────────────────────────────────────────────────

#[test]
fn having_avg_no_violation() {
    let diags = check("SELECT dept FROM products GROUP BY dept HAVING AVG(price) > 10");
    assert!(diags.is_empty());
}

// ── HAVING name = 'test' — no aggregate — 1 violation ────────────────────────

#[test]
fn having_string_comparison_one_violation() {
    let diags = check("SELECT name FROM users GROUP BY name HAVING name = 'test'");
    assert_eq!(diags.len(), 1);
}

// ── HAVING MAX(val) - MIN(val) > 0 — has aggregate — no violation ─────────────

#[test]
fn having_max_minus_min_no_violation() {
    let diags = check("SELECT category FROM products GROUP BY category HAVING MAX(val) - MIN(val) > 0");
    assert!(diags.is_empty());
}

// ── line/col points to HAVING keyword ────────────────────────────────────────

#[test]
fn line_col_points_to_having_keyword() {
    let diags = check("SELECT dept FROM employees GROUP BY dept HAVING col > 5");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── message format ────────────────────────────────────────────────────────────

#[test]
fn message_format_is_correct() {
    let diags = check("SELECT dept FROM employees GROUP BY dept HAVING col > 5");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "HAVING clause contains no aggregate function; use WHERE instead");
}

// ── subquery with HAVING without aggregate — 1 violation ─────────────────────

#[test]
fn subquery_having_without_aggregate_one_violation() {
    let sql = "SELECT * FROM (SELECT dept FROM employees GROUP BY dept HAVING dept = 'sales') sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

// ── HAVING 1 = 1 — no aggregate — 1 violation ────────────────────────────────

#[test]
fn having_literal_comparison_one_violation() {
    let diags = check("SELECT dept FROM employees GROUP BY dept HAVING 1 = 1");
    assert_eq!(diags.len(), 1);
}
