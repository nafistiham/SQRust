use sqrust_core::FileContext;
use sqrust_rules::layout::single_space_after_comma::SingleSpaceAfterComma;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SingleSpaceAfterComma.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SingleSpaceAfterComma.name(), "Layout/SingleSpaceAfterComma");
}

#[test]
fn single_space_after_comma_has_no_violation() {
    let diags = check("SELECT a, b\n");
    assert!(diags.is_empty());
}

#[test]
fn missing_space_after_comma_produces_one_violation() {
    let diags = check("SELECT a,b\n");
    assert_eq!(diags.len(), 1);
}

#[test]
fn extra_space_after_comma_produces_one_violation() {
    let diags = check("SELECT a,  b\n");
    assert_eq!(diags.len(), 1);
}

#[test]
fn trailing_comma_at_end_of_line_has_no_violation() {
    // Comma right before newline — trailing comma, no space required
    let diags = check("SELECT a,\nb\n");
    assert!(diags.is_empty());
}

#[test]
fn comma_inside_single_quoted_string_has_no_violation() {
    let diags = check("SELECT 'a,b'\n");
    assert!(diags.is_empty());
}

#[test]
fn comma_inside_line_comment_has_no_violation() {
    let diags = check("SELECT 1 -- a,b\n");
    assert!(diags.is_empty());
}

#[test]
fn comma_inside_block_comment_has_no_violation() {
    let diags = check("SELECT /* a,b */ 1\n");
    assert!(diags.is_empty());
}

#[test]
fn multiple_bad_commas_produce_multiple_violations() {
    let diags = check("SELECT a,b, c,d\n");
    // commas at positions: a,b (missing), c,d (missing) — 2 violations
    assert_eq!(diags.len(), 2);
}

#[test]
fn correct_line_number_for_comma_on_second_line() {
    let diags = check("SELECT a, b\nFROM t WHERE x IN (1,2)\n");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn correct_col_number_for_comma() {
    // "SELECT a,b" — comma is at col 9 (1-indexed)
    let diags = check("SELECT a,b\n");
    assert_eq!(diags[0].col, 9);
}

#[test]
fn file_with_only_correct_comma_usage_has_no_violations() {
    let diags = check("SELECT a, b, c, d\nFROM t\nWHERE x IN (1, 2, 3)\n");
    assert!(diags.is_empty());
}

#[test]
fn correct_message_text() {
    let diags = check("SELECT a,b\n");
    assert_eq!(diags[0].message, "Expected single space after comma");
}
