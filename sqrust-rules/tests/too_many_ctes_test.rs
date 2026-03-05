use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::too_many_ctes::TooManyCtes;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    TooManyCtes::default().check(&c)
}

fn check_with(sql: &str, max_ctes: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    TooManyCtes { max_ctes }.check(&c)
}

/// Build a SQL string with `n` CTEs like:
///   WITH cte1 AS (SELECT 1), cte2 AS (SELECT 2), ... SELECT * FROM cte1
fn make_ctes(n: usize) -> String {
    if n == 0 {
        return "SELECT 1".to_string();
    }
    let cte_parts: Vec<String> = (1..=n)
        .map(|i| format!("cte{i} AS (SELECT {i})"))
        .collect();
    format!("WITH {} SELECT * FROM cte1", cte_parts.join(", "))
}

// ── rule name ────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(TooManyCtes::default().name(), "TooManyCtes");
}

// ── default max_ctes ──────────────────────────────────────────────────────────

#[test]
fn default_max_ctes_is_five() {
    assert_eq!(TooManyCtes::default().max_ctes, 5);
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("WITH AS SELECT BROKEN FROM");
    assert!(diags.is_empty());
}

// ── 0 CTEs — no violation ─────────────────────────────────────────────────────

#[test]
fn zero_ctes_no_violation() {
    let diags = check("SELECT 1");
    assert!(diags.is_empty());
}

// ── 3 CTEs with default max 5 — no violation ─────────────────────────────────

#[test]
fn three_ctes_default_max_no_violation() {
    let diags = check(&make_ctes(3));
    assert!(diags.is_empty());
}

// ── 5 CTEs with default max 5 — equal is OK — no violation ───────────────────

#[test]
fn five_ctes_at_default_max_no_violation() {
    let diags = check(&make_ctes(5));
    assert!(diags.is_empty());
}

// ── 6 CTEs with default max 5 — 1 violation ──────────────────────────────────

#[test]
fn six_ctes_over_default_max_one_violation() {
    let diags = check(&make_ctes(6));
    assert_eq!(diags.len(), 1);
}

// ── custom max_ctes: 2 with 3 CTEs — 1 violation ─────────────────────────────

#[test]
fn custom_max_2_three_ctes_one_violation() {
    let diags = check_with(&make_ctes(3), 2);
    assert_eq!(diags.len(), 1);
}

// ── custom max_ctes: 2 with 2 CTEs — no violation ────────────────────────────

#[test]
fn custom_max_2_two_ctes_no_violation() {
    let diags = check_with(&make_ctes(2), 2);
    assert!(diags.is_empty());
}

// ── custom max_ctes: 0 with 1 CTE — 1 violation ──────────────────────────────

#[test]
fn custom_max_0_one_cte_one_violation() {
    let diags = check_with(&make_ctes(1), 0);
    assert_eq!(diags.len(), 1);
}

// ── message contains actual count and max ────────────────────────────────────

#[test]
fn message_contains_count_and_max() {
    let diags = check(&make_ctes(6));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains('6'), "message should contain the CTE count");
    assert!(diags[0].message.contains('5'), "message should contain the max");
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_is_nonzero() {
    let diags = check(&make_ctes(6));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── 1 CTE with default max — no violation ────────────────────────────────────

#[test]
fn one_cte_default_max_no_violation() {
    let diags = check(&make_ctes(1));
    assert!(diags.is_empty());
}

// ── 10 CTEs with default max — 1 violation ───────────────────────────────────

#[test]
fn ten_ctes_default_max_one_violation() {
    let diags = check(&make_ctes(10));
    assert_eq!(diags.len(), 1);
}
