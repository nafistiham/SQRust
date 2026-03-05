use sqrust_core::FileContext;
use sqrust_rules::layout::no_double_spaces::NoDoubleSpaces;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoDoubleSpaces.name(), "Layout/NoDoubleSpaces");
}

// ── Parse errors ──────────────────────────────────────────────────────────────

#[test]
fn parse_error_still_checks_source() {
    // Even with a parse error, the rule checks the source text.
    // "SELECT  FROM FROM" has a parse error AND a double space — should flag it.
    let diags = NoDoubleSpaces.check(&ctx("SELECT  FROM FROM"));
    assert_eq!(diags.len(), 1);
}

// ── Single violation ──────────────────────────────────────────────────────────

#[test]
fn double_space_between_select_and_col_produces_one_violation() {
    let diags = NoDoubleSpaces.check(&ctx("SELECT  col FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn single_space_produces_no_violations() {
    let diags = NoDoubleSpaces.check(&ctx("SELECT col FROM t"));
    assert!(diags.is_empty());
}

// ── Multiple violations ───────────────────────────────────────────────────────

#[test]
fn three_double_spaces_produce_three_violations() {
    let diags = NoDoubleSpaces.check(&ctx("SELECT  col  FROM  t"));
    assert_eq!(diags.len(), 3);
}

// ── Skip regions ─────────────────────────────────────────────────────────────

#[test]
fn double_space_in_line_comment_produces_no_violation() {
    let diags = NoDoubleSpaces.check(&ctx("SELECT 1 --  double space in comment"));
    assert!(diags.is_empty());
}

#[test]
fn double_space_inside_single_quoted_string_produces_no_violation() {
    let diags = NoDoubleSpaces.check(&ctx("SELECT 'a  b' FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn double_space_inside_block_comment_produces_no_violation() {
    let diags = NoDoubleSpaces.check(&ctx("SELECT 1 /*  block comment  */ FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn double_space_inside_double_quoted_identifier_produces_no_violation() {
    let diags = NoDoubleSpaces.check(&ctx(r#"SELECT "col  name" FROM t"#));
    assert!(diags.is_empty());
}

// ── Indentation ───────────────────────────────────────────────────────────────

#[test]
fn leading_spaces_on_line_produce_no_violation() {
    // Two spaces at line start = indentation, not a double space violation
    let diags = NoDoubleSpaces.check(&ctx("SELECT 1\n  FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn deeper_indentation_produces_no_violation() {
    let diags = NoDoubleSpaces.check(&ctx("SELECT\n    col\n  FROM t"));
    assert!(diags.is_empty());
}

// ── Triple/long runs ──────────────────────────────────────────────────────────

#[test]
fn triple_space_produces_one_violation_not_two() {
    // Three consecutive spaces = one violation flagged at the start of the run
    let diags = NoDoubleSpaces.check(&ctx("SELECT   col"));
    assert_eq!(diags.len(), 1);
}

// ── Line and column ───────────────────────────────────────────────────────────

#[test]
fn violation_reports_correct_line_and_col() {
    // "SELECT  col" — double space starts at byte offset 6 (col 7, line 1)
    let diags = NoDoubleSpaces.check(&ctx("SELECT  col"));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 7);
}

#[test]
fn violation_on_second_line_reports_correct_line_and_col() {
    // Line 2: "FROM  t" — double space at col 5
    let diags = NoDoubleSpaces.check(&ctx("SELECT col\nFROM  t"));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
    assert_eq!(diags[0].col, 5);
}

// ── Message ───────────────────────────────────────────────────────────────────

#[test]
fn correct_message_text() {
    let diags = NoDoubleSpaces.check(&ctx("SELECT  col"));
    assert_eq!(diags[0].message, "Multiple consecutive spaces found; use a single space");
}

// ── Fix ───────────────────────────────────────────────────────────────────────

#[test]
fn fix_collapses_double_spaces_to_single() {
    let c = ctx("SELECT  col FROM t");
    let fixed = NoDoubleSpaces.fix(&c).expect("fix should return Some");
    assert_eq!(fixed, "SELECT col FROM t");
}

#[test]
fn fix_preserves_indentation() {
    let c = ctx("SELECT 1\n  FROM t");
    // No double spaces in code region, fix should return None
    let result = NoDoubleSpaces.fix(&c);
    assert!(result.is_none());
}

#[test]
fn fix_preserves_spaces_inside_strings() {
    let c = ctx("SELECT 'a  b'  FROM t");
    let fixed = NoDoubleSpaces.fix(&c).expect("fix should return Some");
    assert_eq!(fixed, "SELECT 'a  b' FROM t");
}

#[test]
fn fix_returns_none_when_no_changes_needed() {
    let c = ctx("SELECT col FROM t");
    let result = NoDoubleSpaces.fix(&c);
    assert!(result.is_none());
}

#[test]
fn fix_collapses_triple_space_to_single() {
    let c = ctx("SELECT   col");
    let fixed = NoDoubleSpaces.fix(&c).expect("fix should return Some");
    assert_eq!(fixed, "SELECT col");
}
