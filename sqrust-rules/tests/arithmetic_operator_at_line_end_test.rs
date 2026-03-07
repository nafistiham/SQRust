use sqrust_core::FileContext;
use sqrust_rules::layout::arithmetic_operator_at_line_end::ArithmeticOperatorAtLineEnd;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ArithmeticOperatorAtLineEnd.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    let diags = check("SELECT a +\nFROM t");
    assert_eq!(diags[0].rule, "Layout/ArithmeticOperatorAtLineEnd");
}

#[test]
fn plus_at_line_end_one_violation() {
    let diags = check("SELECT a +\nFROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn minus_at_line_end_one_violation() {
    let diags = check("SELECT a -\nb");
    assert_eq!(diags.len(), 1);
}

#[test]
fn division_at_line_end_one_violation() {
    let diags = check("SELECT a /\nb");
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_trailing_operator_no_violation() {
    // All on one line — no trailing operator
    let diags = check("SELECT a + b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn star_at_line_end_no_violation() {
    // Asterisk is exempt (SELECT *, COUNT(*))
    let diags = check("SELECT *\nFROM t");
    assert!(diags.is_empty());
}

#[test]
fn dash_dash_comment_no_violation() {
    // -- comment line should not be flagged
    let diags = check("SELECT a -- comment\nFROM t");
    assert!(diags.is_empty());
}

#[test]
fn operator_inside_string_no_violation() {
    // + is inside a string literal, should not be flagged
    let diags = check("SELECT 'x +'\nFROM t");
    assert!(diags.is_empty());
}

#[test]
fn two_trailing_operators_two_violations() {
    let sql = "SELECT a +\nb -\nc";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_contains_operator_or_line_end() {
    let diags = check("SELECT a +\nFROM t");
    let msg = &diags[0].message;
    // Message should mention line end or the operator concept
    assert!(
        msg.contains("line end") || msg.contains('+') || msg.contains("operator"),
        "Unexpected message: {}",
        msg
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT a +\nFROM t");
    assert!(diags[0].line > 0);
    assert!(diags[0].col > 0);
}

#[test]
fn col_points_to_operator_position() {
    // "SELECT a +" — the + is at col 10 (1-indexed)
    let diags = check("SELECT a +\nFROM t");
    assert_eq!(diags[0].col, 10);
}

#[test]
fn leading_operator_no_flag() {
    // A line that starts with + (continuation) — no flag, it's at start not end
    let diags = check("SELECT a\n  + b\nFROM t");
    assert!(diags.is_empty());
}

#[test]
fn and_or_at_line_end_no_violation() {
    // AND/OR at line end — that's for LeadingOperator rule, not this one
    let diags = check("SELECT a FROM t WHERE x = 1 AND\n  y = 2");
    assert!(diags.is_empty());
}

#[test]
fn double_dash_at_end_no_violation() {
    // A line ending in "--" is a comment start, should not be flagged
    let diags = check("SELECT a --\nFROM t");
    assert!(diags.is_empty());
}
