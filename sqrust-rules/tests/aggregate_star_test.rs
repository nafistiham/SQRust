use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::aggregate_star::AggregateStar;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    AggregateStar.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(AggregateStar.name(), "Structure/AggregateStar");
}

// ── SUM(*) — 1 violation ──────────────────────────────────────────────────────

#[test]
fn sum_star_violation() {
    let diags = check("SELECT SUM(*) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── AVG(*) — 1 violation ──────────────────────────────────────────────────────

#[test]
fn avg_star_violation() {
    let diags = check("SELECT AVG(*) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── MIN(*) — 1 violation ──────────────────────────────────────────────────────

#[test]
fn min_star_violation() {
    let diags = check("SELECT MIN(*) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── MAX(*) — 1 violation ──────────────────────────────────────────────────────

#[test]
fn max_star_violation() {
    let diags = check("SELECT MAX(*) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── COUNT(*) — 0 violations (valid SQL) ──────────────────────────────────────

#[test]
fn count_star_no_violation() {
    let diags = check("SELECT COUNT(*) FROM t");
    assert!(diags.is_empty(), "COUNT(*) should not be flagged");
}

// ── SUM(amount) — 0 violations ───────────────────────────────────────────────

#[test]
fn sum_column_no_violation() {
    let diags = check("SELECT SUM(amount) FROM t");
    assert!(diags.is_empty());
}

// ── sum(*) lowercase — 1 violation (case-insensitive) ────────────────────────

#[test]
fn sum_star_case_insensitive() {
    let diags = check("SELECT sum(*) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── SUM(*), AVG(*) — 2 violations ────────────────────────────────────────────

#[test]
fn multiple_violations() {
    let diags = check("SELECT SUM(*), AVG(*) FROM t");
    assert_eq!(diags.len(), 2);
}

// ── message mentions the function name ───────────────────────────────────────

#[test]
fn sum_star_message_content() {
    let diags = check("SELECT SUM(*) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("SUM"),
        "message should mention SUM: {}",
        diags[0].message
    );
}

// ── SUM(*) inside a string literal — 0 violations ────────────────────────────

#[test]
fn sum_star_in_string_no_violation() {
    let diags = check("SELECT 'SUM(*) example' FROM t");
    assert!(diags.is_empty(), "SUM(*) inside string should not be flagged");
}

// ── SUM(*) in a line comment — 0 violations ──────────────────────────────────

#[test]
fn sum_star_in_comment_no_violation() {
    let diags = check("-- SUM(*)\nSELECT a FROM t");
    assert!(diags.is_empty(), "SUM(*) inside comment should not be flagged");
}

// ── source-level scan works even with parse errors ───────────────────────────

#[test]
fn parse_error_still_scans() {
    // Source-level scan does not require a successful parse.
    let diags = check("SELECT SUM(*) FROM @@invalid_token");
    // May parse or not, but if source has SUM(*) it should flag it.
    assert_eq!(diags.len(), 1);
}

// ── line and col are non-zero ─────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT SUM(*) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line should be >= 1");
    assert!(diags[0].col >= 1, "col should be >= 1");
}

// ── STDDEV(*) — 1 violation ───────────────────────────────────────────────────

#[test]
fn stddev_star_violation() {
    let diags = check("SELECT STDDEV(*) FROM t");
    assert_eq!(diags.len(), 1);
}
