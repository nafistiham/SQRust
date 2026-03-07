use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::zero_limit_clause::ZeroLimitClause;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    ZeroLimitClause.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(ZeroLimitClause.name(), "Structure/ZeroLimitClause");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM LIMIT");
    assert!(diags.is_empty());
}

// ── no LIMIT — no violation ───────────────────────────────────────────────────

#[test]
fn no_limit_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

// ── LIMIT 1 — no violation ────────────────────────────────────────────────────

#[test]
fn limit_one_no_violation() {
    let diags = check("SELECT id FROM t LIMIT 1");
    assert!(diags.is_empty());
}

// ── LIMIT 0 — 1 violation ─────────────────────────────────────────────────────

#[test]
fn limit_zero_one_violation() {
    let diags = check("SELECT id FROM t LIMIT 0");
    assert_eq!(diags.len(), 1);
}

// ── limit 0 lowercase — 1 violation ──────────────────────────────────────────

#[test]
fn limit_zero_case_insensitive() {
    let diags = check("select id from t limit 0");
    assert_eq!(diags.len(), 1);
}

// ── LIMIT 0 in subquery — flagged ─────────────────────────────────────────────

#[test]
fn limit_zero_in_subquery_flagged() {
    let diags = check("SELECT * FROM (SELECT id FROM t LIMIT 0) sub");
    assert_eq!(diags.len(), 1);
}

// ── LIMIT 10 — no violation ───────────────────────────────────────────────────

#[test]
fn limit_ten_no_violation() {
    let diags = check("SELECT id FROM t LIMIT 10");
    assert!(diags.is_empty());
}

// ── two queries both LIMIT 0 — 2 violations ──────────────────────────────────

#[test]
fn two_queries_both_limit_zero_two_violations() {
    let diags = check("SELECT a FROM t1 LIMIT 0; SELECT b FROM t2 LIMIT 0");
    assert_eq!(diags.len(), 2);
}

// ── message mentions empty ────────────────────────────────────────────────────

#[test]
fn message_mentions_empty() {
    let diags = check("SELECT id FROM t LIMIT 0");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_lowercase().contains("empty"),
        "expected message to mention empty, got: {}",
        diags[0].message
    );
}

// ── line is nonzero ───────────────────────────────────────────────────────────

#[test]
fn line_nonzero() {
    let diags = check("SELECT id FROM t LIMIT 0");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

// ── col is nonzero ────────────────────────────────────────────────────────────

#[test]
fn col_nonzero() {
    let diags = check("SELECT id FROM t LIMIT 0");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

// ── LIMIT 0 OFFSET 5 — still flagged ─────────────────────────────────────────

#[test]
fn limit_with_offset_zero_still_flagged() {
    let diags = check("SELECT id FROM t LIMIT 0 OFFSET 5");
    assert_eq!(diags.len(), 1);
}
