use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::natural_join::NaturalJoin;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    NaturalJoin.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(NaturalJoin.name(), "NaturalJoin");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM JOIN BROKEN WHERE");
    assert!(diags.is_empty());
}

// ── NATURAL JOIN — 1 violation ────────────────────────────────────────────────

#[test]
fn natural_join_one_violation() {
    let diags = check("SELECT * FROM a NATURAL JOIN b");
    assert_eq!(diags.len(), 1);
}

// ── INNER JOIN with ON — no violation ────────────────────────────────────────

#[test]
fn inner_join_no_violation() {
    let diags = check("SELECT * FROM a INNER JOIN b ON a.id = b.id");
    assert!(diags.is_empty());
}

// ── LEFT JOIN with ON — no violation ─────────────────────────────────────────

#[test]
fn left_join_no_violation() {
    let diags = check("SELECT * FROM a LEFT JOIN b ON a.id = b.id");
    assert!(diags.is_empty());
}

// ── CROSS JOIN — no violation ─────────────────────────────────────────────────

#[test]
fn cross_join_no_violation() {
    let diags = check("SELECT * FROM a CROSS JOIN b");
    assert!(diags.is_empty());
}

// ── no JOIN at all — no violation ────────────────────────────────────────────

#[test]
fn no_join_no_violation() {
    let diags = check("SELECT id FROM t WHERE id = 1");
    assert!(diags.is_empty());
}

// ── two NATURAL JOINs — 2 violations ─────────────────────────────────────────

#[test]
fn two_natural_joins_two_violations() {
    let diags = check("SELECT * FROM a NATURAL JOIN b NATURAL JOIN c");
    assert_eq!(diags.len(), 2);
}

// ── NATURAL LEFT JOIN — 1 violation if sqlparser parses it ───────────────────

#[test]
fn natural_left_join_violation() {
    // sqlparser-rs represents NATURAL LEFT JOIN as LeftOuter(JoinConstraint::Natural).
    // If the GenericDialect parses it, expect 1 violation; if it's a parse error,
    // the rule returns 0 (safe either way).
    let diags = check("SELECT * FROM a NATURAL LEFT JOIN b");
    // Either 1 violation (parsed) or 0 (parse error). We assert parsed case is flagged.
    // The real assertion is: no panic, and count is 0 or 1.
    assert!(diags.len() <= 1);
    // If parsed successfully, it must flag it.
    let ctx_val = ctx("SELECT * FROM a NATURAL LEFT JOIN b");
    if ctx_val.parse_errors.is_empty() {
        assert_eq!(diags.len(), 1);
    }
}

// ── message contains useful text ─────────────────────────────────────────────

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT * FROM a NATURAL JOIN b");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("NATURAL JOIN") || msg.contains("natural join") || msg.contains("Natural"),
        "message should mention NATURAL JOIN, got: {msg}"
    );
    assert!(
        msg.contains("explicit") || msg.contains("ON") || msg.contains("convention"),
        "message should recommend explicit join or mention convention, got: {msg}"
    );
}

// ── line/col nonzero ─────────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM a NATURAL JOIN b");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── NATURAL JOIN in subquery — 1 violation ───────────────────────────────────

#[test]
fn natural_join_in_subquery_violation() {
    let sql = "SELECT * FROM (SELECT * FROM a NATURAL JOIN b) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

// ── NATURAL JOIN in CTE — 1 violation ────────────────────────────────────────

#[test]
fn natural_join_in_cte_violation() {
    let sql = "WITH cte AS (SELECT * FROM a NATURAL JOIN b) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
