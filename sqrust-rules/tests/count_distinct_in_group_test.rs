use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::count_distinct_in_group::CountDistinctInGroup;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    CountDistinctInGroup.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        CountDistinctInGroup.name(),
        "Structure/CountDistinctInGroup"
    );
}

// ── core violation ────────────────────────────────────────────────────────────

#[test]
fn count_distinct_grouped_col_violation() {
    let diags = check("SELECT a, COUNT(DISTINCT a) FROM t GROUP BY a");
    assert_eq!(diags.len(), 1);
}

// ── counting a different column — no violation ────────────────────────────────

#[test]
fn count_distinct_different_col_no_violation() {
    let diags = check("SELECT a, COUNT(DISTINCT b) FROM t GROUP BY a");
    assert!(diags.is_empty());
}

// ── COUNT(*) — no violation ───────────────────────────────────────────────────

#[test]
fn count_star_group_by_no_violation() {
    let diags = check("SELECT a, COUNT(*) FROM t GROUP BY a");
    assert!(diags.is_empty());
}

// ── COUNT(DISTINCT) without GROUP BY — no violation ───────────────────────────

#[test]
fn count_distinct_no_group_by_no_violation() {
    let diags = check("SELECT COUNT(DISTINCT a) FROM t");
    assert!(diags.is_empty());
}

// ── multiple GROUP BY cols, one of them counted — violation ───────────────────

#[test]
fn multiple_group_cols_one_counted_violation() {
    let diags = check("SELECT a, b, COUNT(DISTINCT a) FROM t GROUP BY a, b");
    assert_eq!(diags.len(), 1);
}

// ── multiple GROUP BY cols, none of them counted — no violation ───────────────

#[test]
fn multiple_group_cols_none_counted_no_violation() {
    let diags = check("SELECT a, b, COUNT(DISTINCT c) FROM t GROUP BY a, b");
    assert!(diags.is_empty());
}

// ── message mentions the column ───────────────────────────────────────────────

#[test]
fn message_mentions_column() {
    let diags = check("SELECT a, COUNT(DISTINCT a) FROM t GROUP BY a");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("a"),
        "expected message to mention column 'a', got: {}",
        diags[0].message
    );
}

// ── parse error — no violations ───────────────────────────────────────────────

#[test]
fn parse_error_no_violations() {
    let diags = check("SELECT FROM FROM WHERE");
    assert!(diags.is_empty());
}

// ── line and col are nonzero ──────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT a, COUNT(DISTINCT a) FROM t GROUP BY a");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── SUM(DISTINCT) is not flagged — only COUNT ─────────────────────────────────

#[test]
fn sum_distinct_not_flagged() {
    let diags = check("SELECT a, SUM(DISTINCT a) FROM t GROUP BY a");
    assert!(diags.is_empty());
}

// ── case insensitive matching ─────────────────────────────────────────────────

#[test]
fn count_distinct_case_insensitive() {
    let diags = check("SELECT A, count(distinct A) FROM t GROUP BY A");
    assert_eq!(diags.len(), 1);
}

// ── CTE with COUNT(DISTINCT) violation ───────────────────────────────────────

#[test]
fn cte_with_count_distinct_violation() {
    let diags = check(
        "WITH cte AS (SELECT a, COUNT(DISTINCT a) FROM t GROUP BY a) SELECT * FROM cte",
    );
    assert_eq!(diags.len(), 1);
}
