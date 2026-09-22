use sqrust_core::FileContext;
use sqrust_rules::layout::trailing_whitespace::TrailingWhitespace;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    TrailingWhitespace.check(&ctx)
}

#[test]
fn clean_sql_has_no_violations() {
    let diags = check("SELECT id\nFROM users\n");
    assert!(diags.is_empty());
}

#[test]
fn trailing_spaces_flagged_on_correct_line() {
    let diags = check("SELECT id   \nFROM users\n");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
}

#[test]
fn trailing_tab_is_flagged() {
    let diags = check("SELECT id\t\nFROM users\n");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
}

#[test]
fn multiple_trailing_lines_all_flagged() {
    let diags = check("SELECT id   \nFROM users  \nWHERE 1=1\n");
    assert_eq!(diags.len(), 2);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[1].line, 2);
}

#[test]
fn col_points_to_first_trailing_whitespace() {
    // "SELECT id   " — content is 9 chars, first trailing space is col 10
    let diags = check("SELECT id   \nFROM users\n");
    assert_eq!(diags[0].col, 10);
}

#[test]
fn empty_sql_has_no_violations() {
    assert!(check("").is_empty());
}

#[test]
fn rule_name_is_correct() {
    let ctx = FileContext::from_source("SELECT 1   \n", "test.sql");
    let diags = TrailingWhitespace.check(&ctx);
    assert_eq!(diags[0].rule, "Layout/TrailingWhitespace");
}

#[test]
fn fix_removes_trailing_whitespace() {
    use sqrust_core::Rule;
    let ctx = FileContext::from_source("SELECT id   \nFROM users  \n", "test.sql");
    let fixed = TrailingWhitespace.fix(&ctx).expect("fix should be available");
    assert_eq!(fixed, "SELECT id\nFROM users\n");
}

#[test]
fn fix_returns_none_when_already_clean() {
    use sqrust_core::Rule;
    let ctx = FileContext::from_source("SELECT id\nFROM users\n", "test.sql");
    assert!(TrailingWhitespace.fix(&ctx).is_none());
}

#[test]
fn fix_preserves_crlf_line_endings() {
    use sqrust_core::Rule;
    let ctx = FileContext::from_source("SELECT id   \r\nFROM users  \r\n", "test.sql");
    let fixed = TrailingWhitespace.fix(&ctx).expect("fix should be available");
    assert_eq!(fixed, "SELECT id\r\nFROM users\r\n");
}

#[test]
fn crlf_line_with_trailing_space_is_flagged() {
    let diags = check("SELECT id \r\nFROM users\r\n");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
}

#[test]
fn whitespace_only_line_is_flagged() {
    let diags = check("SELECT id\n   \nFROM users\n");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn last_line_without_newline_still_flagged() {
    let diags = check("SELECT id\nFROM users\t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}
