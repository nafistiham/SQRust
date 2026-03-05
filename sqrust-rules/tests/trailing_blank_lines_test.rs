use sqrust_core::FileContext;
use sqrust_rules::layout::trailing_blank_lines::TrailingBlankLines;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    TrailingBlankLines.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(TrailingBlankLines.name(), "Layout/TrailingBlankLines");
}

#[test]
fn one_blank_line_after_content_produces_one_violation() {
    let diags = check("SELECT 1\n\n");
    assert_eq!(diags.len(), 1);
}

#[test]
fn single_trailing_newline_has_no_violation() {
    // "SELECT 1\n" — proper final newline, not a blank line
    let diags = check("SELECT 1\n");
    assert!(diags.is_empty());
}

#[test]
fn no_trailing_newline_has_no_violation() {
    let diags = check("SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn two_blank_lines_after_content_produces_one_violation() {
    // Report once even though two extra blank lines
    let diags = check("SELECT 1\n\n\n");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_lines_with_single_trailing_newline_has_no_violation() {
    let diags = check("SELECT 1\nSELECT 2\n");
    assert!(diags.is_empty());
}

#[test]
fn whitespace_only_trailing_line_produces_one_violation() {
    let diags = check("SELECT 1\n   \n");
    assert_eq!(diags.len(), 1);
}

#[test]
fn single_newline_file_has_no_violation() {
    // Edge case: just a newline — treat as empty file, no violation
    let diags = check("\n");
    assert!(diags.is_empty());
}

#[test]
fn blank_lines_between_statements_not_violations() {
    // Blank lines in the middle are fine — only trailing ones matter
    let diags = check("SELECT 1\n\n\nSELECT 2\n");
    assert!(diags.is_empty());
}

#[test]
fn fix_removes_trailing_blank_line() {
    let ctx = FileContext::from_source("SELECT 1\n\n", "test.sql");
    let fixed = TrailingBlankLines.fix(&ctx).expect("fix should return Some");
    assert_eq!(fixed, "SELECT 1\n");
}

#[test]
fn fix_returns_none_when_no_trailing_blank_lines() {
    let ctx = FileContext::from_source("SELECT 1\n", "test.sql");
    let result = TrailingBlankLines.fix(&ctx);
    assert!(result.is_none());
}

#[test]
fn correct_message_text() {
    let diags = check("SELECT 1\n\n");
    assert_eq!(diags[0].message, "File has trailing blank line(s)");
}

#[test]
fn multiple_whitespace_only_trailing_lines_produces_one_violation() {
    let diags = check("SELECT 1\n  \n  \n");
    assert_eq!(diags.len(), 1);
}

#[test]
fn violation_reports_first_trailing_blank_line_number() {
    // "SELECT 1\n\n\n" — line 1: "SELECT 1", line 2: "", line 3: ""
    // First trailing blank line is line 2
    let diags = check("SELECT 1\n\n\n");
    assert_eq!(diags[0].line, 2);
}
