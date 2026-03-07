use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::mixed_aggregate_and_columns::MixedAggregateAndColumns;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    MixedAggregateAndColumns.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        MixedAggregateAndColumns.name(),
        "Structure/MixedAggregateAndColumns"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM GROUP BROKEN BY");
    assert!(diags.is_empty());
}

// ── only aggregate — no violation ─────────────────────────────────────────────

#[test]
fn only_aggregate_no_violation() {
    let diags = check("SELECT COUNT(*) FROM t");
    assert!(diags.is_empty());
}

// ── only columns — no violation ───────────────────────────────────────────────

#[test]
fn only_columns_no_violation() {
    let diags = check("SELECT id, name FROM t");
    assert!(diags.is_empty());
}

// ── aggregate with GROUP BY — no violation ────────────────────────────────────

#[test]
fn aggregate_with_group_by_no_violation() {
    let diags = check("SELECT dept, COUNT(*) FROM t GROUP BY dept");
    assert!(diags.is_empty());
}

// ── mixed without GROUP BY → 1 violation ─────────────────────────────────────

#[test]
fn mixed_without_group_by_one_violation() {
    let diags = check("SELECT name, COUNT(*) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── SUM without GROUP BY → 1 violation ───────────────────────────────────────

#[test]
fn sum_without_group_by_flagged() {
    let diags = check("SELECT id, SUM(amount) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── AVG without GROUP BY → 1 violation ───────────────────────────────────────

#[test]
fn avg_without_group_by_flagged() {
    let diags = check("SELECT category, AVG(price) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── COUNT(*) is an aggregate ──────────────────────────────────────────────────

#[test]
fn count_star_is_aggregate() {
    let diags = check("SELECT id, COUNT(*) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── MAX without GROUP BY → 1 violation ───────────────────────────────────────

#[test]
fn max_without_group_by_flagged() {
    let diags = check("SELECT product, MAX(price) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── message mentions "aggregate" ──────────────────────────────────────────────

#[test]
fn message_mentions_aggregate() {
    let diags = check("SELECT name, COUNT(*) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("aggregate"),
        "message should contain 'aggregate', got: {}",
        diags[0].message
    );
}

// ── line is non-zero ──────────────────────────────────────────────────────────

#[test]
fn line_nonzero() {
    let diags = check("SELECT name, COUNT(*) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

// ── col is non-zero ───────────────────────────────────────────────────────────

#[test]
fn col_nonzero() {
    let diags = check("SELECT name, COUNT(*) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

// ── subquery checked independently ───────────────────────────────────────────
// Outer SELECT is fine (only columns, no aggregate).
// Inner subquery SELECT mixes aggregate + bare column → 1 violation.

#[test]
fn subquery_checked_independently() {
    let sql = "SELECT id FROM \
               (SELECT name, COUNT(*) FROM orders) AS sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
