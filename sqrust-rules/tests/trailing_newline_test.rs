use sqrust_core::FileContext;
use sqrust_rules::layout::trailing_newline::TrailingNewline;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    TrailingNewline.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(TrailingNewline.name(), "Layout/TrailingNewline");
}

#[test]
fn empty_file_has_no_violations() {
    assert!(check("").is_empty());
}

#[test]
fn single_line_ending_with_newline_has_no_violation() {
    let diags = check("SELECT 1\n");
    assert!(diags.is_empty());
}

#[test]
fn single_line_not_ending_with_newline_has_one_violation() {
    let diags = check("SELECT 1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiline_file_ending_with_newline_has_no_violation() {
    let diags = check("SELECT id\nFROM users\nWHERE id = 1\n");
    assert!(diags.is_empty());
}

#[test]
fn multiline_file_not_ending_with_newline_has_one_violation_with_correct_line() {
    let diags = check("SELECT id\nFROM users\nWHERE id = 1");
    assert_eq!(diags.len(), 1);
    // 3 lines total; violation should be on line 3
    assert_eq!(diags[0].line, 3);
}

#[test]
fn correct_message_text() {
    let diags = check("SELECT 1");
    assert_eq!(diags[0].message, "File must end with a newline");
}

#[test]
fn file_ending_with_double_newline_has_no_violation() {
    let diags = check("SELECT 1\n\n");
    assert!(diags.is_empty());
}

#[test]
fn crlf_file_ending_with_newline_has_no_violation() {
    let diags = check("SELECT 1\r\nFROM users\r\n");
    assert!(diags.is_empty());
}

#[test]
fn crlf_file_not_ending_with_newline_flags_last_line() {
    let diags = check("SELECT 1\r\nFROM users");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn violation_col_points_just_past_last_character() {
    let diags = check("SELECT 1");
    assert_eq!(diags[0].col, 9);
}

#[test]
fn diagnostic_carries_correct_rule_name() {
    let diags = check("SELECT 1");
    assert_eq!(diags[0].rule, "Layout/TrailingNewline");
}

#[test]
fn rule_is_report_only_no_fix_available() {
    let ctx = FileContext::from_source("SELECT 1", "test.sql");
    assert!(TrailingNewline.fix(&ctx).is_none());
}
