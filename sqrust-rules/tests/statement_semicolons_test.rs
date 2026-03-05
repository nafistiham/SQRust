use sqrust_core::FileContext;
use sqrust_rules::layout::statement_semicolons::StatementSemicolons;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(StatementSemicolons.name(), "Layout/StatementSemicolons");
}

// ── Parse errors ─────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    // Malformed SQL — can't determine statement boundaries, skip entirely
    let diags = StatementSemicolons.check(&ctx("SELECT FROM FROM"));
    assert!(diags.is_empty());
}

// ── With semicolon (no violations) ───────────────────────────────────────────

#[test]
fn single_statement_with_semicolon_produces_no_violation() {
    let diags = StatementSemicolons.check(&ctx("SELECT 1;"));
    assert!(diags.is_empty());
}

#[test]
fn semicolon_followed_by_newline_produces_no_violation() {
    let diags = StatementSemicolons.check(&ctx("SELECT 1;\n"));
    assert!(diags.is_empty());
}

#[test]
fn full_select_with_semicolon_produces_no_violation() {
    let diags = StatementSemicolons.check(&ctx("SELECT col FROM t WHERE id = 1;"));
    assert!(diags.is_empty());
}

#[test]
fn two_statements_both_with_semicolons_produces_no_violation() {
    let diags = StatementSemicolons.check(&ctx("SELECT 1; SELECT 2;"));
    assert!(diags.is_empty());
}

#[test]
fn multiline_statement_with_semicolon_produces_no_violation() {
    let diags = StatementSemicolons.check(&ctx("SELECT\n1\n;"));
    assert!(diags.is_empty());
}

// ── Without semicolon (violations) ───────────────────────────────────────────

#[test]
fn single_statement_without_semicolon_produces_one_violation() {
    let diags = StatementSemicolons.check(&ctx("SELECT 1"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn statement_without_semicolon_with_trailing_newline_produces_one_violation() {
    let diags = StatementSemicolons.check(&ctx("SELECT 1\n"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn full_select_without_semicolon_produces_one_violation() {
    let diags = StatementSemicolons.check(&ctx("SELECT col FROM t WHERE id = 1"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiline_statement_without_semicolon_produces_one_violation() {
    let diags = StatementSemicolons.check(&ctx("SELECT\n1"));
    assert_eq!(diags.len(), 1);
}

// ── Edge cases ────────────────────────────────────────────────────────────────

#[test]
fn empty_source_produces_no_violation() {
    let diags = StatementSemicolons.check(&ctx(""));
    assert!(diags.is_empty());
}

#[test]
fn whitespace_only_source_produces_no_violation() {
    let diags = StatementSemicolons.check(&ctx("   \n   \n  "));
    assert!(diags.is_empty());
}

// ── Violation location ────────────────────────────────────────────────────────

#[test]
fn violation_points_to_last_line_of_sql() {
    // "SELECT 1" — single line, violation on line 1
    let diags = StatementSemicolons.check(&ctx("SELECT 1"));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
}

#[test]
fn violation_on_multiline_points_to_last_non_empty_line() {
    // Last non-empty line is line 2: "1"
    let diags = StatementSemicolons.check(&ctx("SELECT\n1"));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

// ── Message ───────────────────────────────────────────────────────────────────

#[test]
fn correct_message_text() {
    let diags = StatementSemicolons.check(&ctx("SELECT 1"));
    assert_eq!(diags[0].message, "SQL statement is missing a trailing semicolon");
}
