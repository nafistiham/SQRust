use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::aggregate_in_where::AggregateInWhere;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    AggregateInWhere.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(AggregateInWhere.name(), "Structure/AggregateInWhere");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM WHERE");
    assert!(diags.is_empty());
}

// ── no aggregate in WHERE — no violation ─────────────────────────────────────

#[test]
fn no_aggregate_in_where_no_violation() {
    let diags = check("SELECT id FROM t WHERE id > 1");
    assert!(diags.is_empty());
}

// ── COUNT(*) in WHERE — 1 violation ──────────────────────────────────────────

#[test]
fn count_in_where_one_violation() {
    let diags = check("SELECT id FROM t WHERE COUNT(*) > 0");
    assert_eq!(diags.len(), 1);
}

// ── SUM in WHERE — 1 violation ────────────────────────────────────────────────

#[test]
fn sum_in_where_one_violation() {
    let diags = check("SELECT id FROM t WHERE SUM(amount) > 100");
    assert_eq!(diags.len(), 1);
}

// ── AVG in WHERE — 1 violation ────────────────────────────────────────────────

#[test]
fn avg_in_where_one_violation() {
    let diags = check("SELECT id FROM t WHERE AVG(price) < 50");
    assert_eq!(diags.len(), 1);
}

// ── MAX in WHERE — 1 violation ────────────────────────────────────────────────

#[test]
fn max_in_where_flagged() {
    let diags = check("SELECT id FROM t WHERE MAX(score) = 100");
    assert_eq!(diags.len(), 1);
}

// ── aggregate in HAVING — no violation ───────────────────────────────────────

#[test]
fn aggregate_in_having_no_violation() {
    let diags = check(
        "SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) > 5",
    );
    assert!(diags.is_empty());
}

// ── aggregate in SELECT — no violation ───────────────────────────────────────

#[test]
fn aggregate_in_select_no_violation() {
    let diags = check("SELECT COUNT(*) FROM t");
    assert!(diags.is_empty());
}

// ── two aggregates in WHERE — 2 violations ───────────────────────────────────

#[test]
fn two_aggregates_in_where_two_violations() {
    let diags = check("SELECT id FROM t WHERE COUNT(*) > 0 AND SUM(x) > 0");
    assert_eq!(diags.len(), 2);
}

// ── message mentions HAVING ───────────────────────────────────────────────────

#[test]
fn message_mentions_having() {
    let diags = check("SELECT id FROM t WHERE COUNT(*) > 0");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_uppercase().contains("HAVING"),
        "expected message to mention HAVING, got: {}",
        diags[0].message
    );
}

// ── line is nonzero ───────────────────────────────────────────────────────────

#[test]
fn line_nonzero() {
    let diags = check("SELECT id FROM t WHERE COUNT(*) > 0");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

// ── col is nonzero ────────────────────────────────────────────────────────────

#[test]
fn col_nonzero() {
    let diags = check("SELECT id FROM t WHERE COUNT(*) > 0");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

// ── aggregate in WHERE of a subquery — flagged ────────────────────────────────

#[test]
fn subquery_where_checked() {
    let sql = "SELECT * FROM (SELECT id FROM t WHERE SUM(amount) > 0) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
