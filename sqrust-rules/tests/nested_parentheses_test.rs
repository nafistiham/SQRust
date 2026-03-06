use sqrust_core::FileContext;
use sqrust_rules::layout::nested_parentheses::NestedParentheses;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    NestedParentheses::default().check(&ctx(sql))
}

fn check_with_max(sql: &str, max_depth: usize) -> Vec<sqrust_core::Diagnostic> {
    NestedParentheses { max_depth }.check(&ctx(sql))
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(NestedParentheses::default().name(), "Layout/NestedParentheses");
}

#[test]
fn default_max_is_five() {
    assert_eq!(NestedParentheses::default().max_depth, 5);
}

// ── No violations ─────────────────────────────────────────────────────────────

#[test]
fn depth_five_at_max_no_violation() {
    // Five levels deep — exactly at the limit, should not violate
    let diags = check("SELECT (((((x))))) FROM t");
    assert!(diags.is_empty(), "depth 5 should not violate with max 5");
}

#[test]
fn depth_three_no_violation() {
    let diags = check("SELECT (((x))) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_parens_no_violation() {
    let diags = check("SELECT id FROM t WHERE id > 1");
    assert!(diags.is_empty());
}

#[test]
fn parens_in_string_no_violation() {
    // Five opening parens inside a string — should not be counted
    let diags = check("SELECT '(((((', 1 FROM t");
    assert!(diags.is_empty(), "parens inside strings should not be counted");
}

#[test]
fn custom_max_3_depth_3_no_violation() {
    let diags = check_with_max("SELECT (((x))) FROM t", 3);
    assert!(diags.is_empty());
}

// ── Violations ────────────────────────────────────────────────────────────────

#[test]
fn depth_six_over_max_one_violation() {
    // Six levels deep — exceeds default max of 5 by one
    let diags = check("SELECT ((((((x)))))) FROM t");
    assert_eq!(diags.len(), 1, "depth 6 should produce exactly 1 violation");
}

#[test]
fn custom_max_3_depth_4_one_violation() {
    let diags = check_with_max("SELECT ((((x)))) FROM t", 3);
    assert_eq!(diags.len(), 1);
}

// ── Message and position ──────────────────────────────────────────────────────

#[test]
fn message_contains_depth_and_max() {
    let diags = check("SELECT ((((((x)))))) FROM t");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    // message should include the actual depth and the max
    assert!(
        msg.contains('6') && msg.contains('5'),
        "message should contain depth 6 and max 5, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT ((((((x)))))) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn violation_at_opening_paren() {
    // "SELECT " is 7 chars. The opening parens are at col 8..13.
    // depth becomes 6 at the 6th '(' — col 13 (1-indexed).
    let diags = check("SELECT ((((((x)))))) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 13, "violation should be at the 6th '(' (col 13)");
}

#[test]
fn sql_function_call_depth_violation() {
    // f(g(h(i(j(k(x)))))) — 6 deep — should produce 1 violation
    let diags = check("SELECT f(g(h(i(j(k(x)))))) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_still_checks_source() {
    // Invalid SQL — parser will fail, but text-based check still works
    let diags = check("THIS IS NOT SQL ((((((x))))))");
    assert_eq!(diags.len(), 1, "text-based rule should work despite parse errors");
}
