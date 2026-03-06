use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::ambiguous_bool_op::AmbiguousBoolOp;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    AmbiguousBoolOp.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(AmbiguousBoolOp.name(), "Ambiguous/AmbiguousBoolOp");
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = AmbiguousBoolOp.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn or_with_direct_and_child_violation() {
    // sqlparser parses AND before OR (SQL precedence), so:
    // a = 1 OR b = 2 AND c = 3  →  OR(a=1, AND(b=2, c=3))
    // OR has a raw AND child on the right → flag
    let diags = check("SELECT * FROM t WHERE a = 1 OR b = 2 AND c = 3");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/AmbiguousBoolOp");
}

#[test]
fn and_with_direct_or_child_violation() {
    // a AND b OR c  →  OR(AND(a,b), c)
    // OR has a raw AND child on the left → flag
    let diags = check("SELECT * FROM t WHERE a = 1 AND b = 2 OR c = 3");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/AmbiguousBoolOp");
}

#[test]
fn parens_around_and_no_violation() {
    // a = 1 OR (b = 2 AND c = 3) — explicit parens make it Nested(AND(...))
    let diags = check("SELECT * FROM t WHERE a = 1 OR (b = 2 AND c = 3)");
    assert!(diags.is_empty());
}

#[test]
fn parens_around_or_no_violation() {
    // (a = 1 OR b = 2) AND c = 3 — explicit parens
    let diags = check("SELECT * FROM t WHERE (a = 1 OR b = 2) AND c = 3");
    assert!(diags.is_empty());
}

#[test]
fn only_or_no_violation() {
    let diags = check("SELECT * FROM t WHERE a = 1 OR b = 2 OR c = 3");
    assert!(diags.is_empty());
}

#[test]
fn only_and_no_violation() {
    let diags = check("SELECT * FROM t WHERE a = 1 AND b = 2 AND c = 3");
    assert!(diags.is_empty());
}

#[test]
fn no_where_no_violation() {
    let diags = check("SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT * FROM t WHERE a = 1 OR b = 2 AND c = 3");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("AND") && diags[0].message.contains("OR"),
        "message was: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM t WHERE a = 1 OR b = 2 AND c = 3");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn or_and_or_multiple_violations() {
    // a OR b AND c OR d AND e
    // sqlparser: OR(OR(a, AND(b,c)), AND(d,e))
    // The outer OR has right=AND(d,e) → 1 violation from outer OR
    // The inner OR has right=AND(b,c) → 1 violation from inner OR
    // Total: 2 violations
    let diags = check("SELECT * FROM t WHERE a = 1 OR b = 2 AND c = 3 OR d = 4 AND e = 5");
    assert!(diags.len() >= 2, "expected at least 2 violations, got {}", diags.len());
}

#[test]
fn having_clause_or_and_violation() {
    let diags = check(
        "SELECT col, COUNT(*) FROM t GROUP BY col HAVING COUNT(*) > 1 OR col = 'x' AND col = 'y'",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn deeply_nested_explicit_parens_no_violation() {
    // (a OR b) AND (c OR d) — all mixed ops are inside Nested(), so no raw mix
    let diags = check("SELECT * FROM t WHERE (a = 1 OR b = 2) AND (c = 3 OR d = 4)");
    assert!(diags.is_empty());
}
