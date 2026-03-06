use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::in_single_value::InSingleValue;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    InSingleValue.check(&ctx)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(InSingleValue.name(), "InSingleValue");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = InSingleValue.check(&ctx);
        assert!(diags.is_empty());
    }
}

// ── basic violation ───────────────────────────────────────────────────────────

#[test]
fn in_single_value_one_violation() {
    let diags = check("SELECT * FROM t WHERE id IN (1)");
    assert_eq!(diags.len(), 1);
}

// ── no violation cases ────────────────────────────────────────────────────────

#[test]
fn in_two_values_no_violation() {
    let diags = check("SELECT * FROM t WHERE id IN (1, 2)");
    assert!(diags.is_empty());
}

#[test]
fn not_in_single_value_no_violation() {
    // NOT IN (x) is different semantics — not equivalent to != x for NULLs
    let diags = check("SELECT * FROM t WHERE id NOT IN (1)");
    assert!(diags.is_empty());
}

#[test]
fn no_in_no_violation() {
    let diags = check("SELECT * FROM t WHERE id = 1");
    assert!(diags.is_empty());
}

#[test]
fn in_single_subquery_no_violation() {
    // IN (SELECT ...) is InSubquery, not InList — should not be flagged
    let diags = check("SELECT * FROM t WHERE id IN (SELECT id FROM u)");
    assert!(diags.is_empty());
}

// ── variant violations ────────────────────────────────────────────────────────

#[test]
fn in_single_string_value_violation() {
    let diags = check("SELECT * FROM t WHERE name IN ('foo')");
    assert_eq!(diags.len(), 1);
}

#[test]
fn in_single_null_violation() {
    let diags = check("SELECT * FROM t WHERE name IN (NULL)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn in_empty_list_no_violation() {
    // Empty IN list — sqlparser may or may not parse this; should not crash
    // If it fails to parse, no violations; if parsed, 0 violations (list.len() == 0)
    let ctx = FileContext::from_source("SELECT * FROM t WHERE id IN ()", "test.sql");
    let diags = InSingleValue.check(&ctx);
    // Either 0 violations or parse error — just must not panic
    let _ = diags;
}

// ── message and position ──────────────────────────────────────────────────────

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT * FROM t WHERE id IN (1)");
    assert_eq!(diags.len(), 1);
    assert!(!diags[0].message.is_empty());
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM t WHERE id IN (1)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── multiple violations ───────────────────────────────────────────────────────

#[test]
fn multiple_in_single_value_two_violations() {
    let diags = check("SELECT * FROM t WHERE id IN (1) AND name IN ('foo')");
    assert_eq!(diags.len(), 2);
}
