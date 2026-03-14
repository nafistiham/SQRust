use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::nested_aggregate::NestedAggregate;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    NestedAggregate.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(NestedAggregate.name(), "Structure/NestedAggregate");
}

// ── basic violations ──────────────────────────────────────────────────────────

#[test]
fn sum_of_count_one_violation() {
    let diags = check("SELECT SUM(COUNT(x)) FROM t GROUP BY y");
    assert_eq!(diags.len(), 1);
}

#[test]
fn max_of_min_one_violation() {
    let diags = check("SELECT MAX(MIN(price)) FROM t GROUP BY cat");
    assert_eq!(diags.len(), 1);
}

#[test]
fn count_of_sum_one_violation() {
    let diags = check("SELECT COUNT(SUM(x)) FROM t GROUP BY y");
    assert_eq!(diags.len(), 1);
}

#[test]
fn avg_of_max_one_violation() {
    let diags = check("SELECT AVG(MAX(score)) FROM t GROUP BY dept");
    assert_eq!(diags.len(), 1);
}

// ── no violations ─────────────────────────────────────────────────────────────

#[test]
fn simple_sum_no_violation() {
    let diags = check("SELECT SUM(x) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn simple_count_star_no_violation() {
    let diags = check("SELECT COUNT(*) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nested_non_aggregate_no_violation() {
    // ABS is not an aggregate function
    let diags = check("SELECT SUM(ABS(x)) FROM t");
    assert!(diags.is_empty());
}

// ── multiple violations ───────────────────────────────────────────────────────

#[test]
fn two_nested_aggregates_two_violations() {
    let diags = check(
        "SELECT SUM(COUNT(x)), MAX(MIN(y)) FROM t GROUP BY z",
    );
    assert_eq!(diags.len(), 2);
}

// ── context locations ─────────────────────────────────────────────────────────

#[test]
fn nested_aggregate_in_having_violation() {
    let diags = check("SELECT y FROM t GROUP BY y HAVING SUM(COUNT(x)) > 5");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nested_aggregate_in_select_violation() {
    let diags = check("SELECT MAX(SUM(amount)) FROM orders GROUP BY region");
    assert_eq!(diags.len(), 1);
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_no_violations() {
    let diags = check("SELECT FROM FROM WHERE");
    assert!(diags.is_empty());
}

// ── message format ────────────────────────────────────────────────────────────

#[test]
fn message_format_correct() {
    let diags = check("SELECT SUM(COUNT(x)) FROM t GROUP BY y");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_lowercase().contains("aggregate"),
        "expected message to mention aggregate, got: {}",
        diags[0].message
    );
}

// ── line/col are nonzero ──────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT SUM(COUNT(x)) FROM t GROUP BY y");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
