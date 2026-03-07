use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::max_identifier_length::MaxIdentifierLength;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    MaxIdentifierLength::default().check(&ctx)
}

fn check_with_max(sql: &str, max_length: usize) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    MaxIdentifierLength { max_length }.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(MaxIdentifierLength::default().name(), "Layout/MaxIdentifierLength");
}

#[test]
fn short_identifier_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty(), "expected no violations but got {:?}", diags.iter().map(|d| &d.message).collect::<Vec<_>>());
}

#[test]
fn identifier_at_limit_no_violation() {
    // Exactly 30 chars — should not be flagged
    let ident = "a".repeat(30);
    let sql = format!("SELECT {} FROM t", ident);
    let diags = check(&sql);
    assert!(diags.is_empty(), "30-char identifier should not be flagged");
}

#[test]
fn identifier_over_limit_one_violation() {
    // 31 chars — should be flagged
    let ident = "a".repeat(31);
    let sql = format!("SELECT {} FROM t", ident);
    let diags = check(&sql);
    assert_eq!(diags.len(), 1, "expected 1 violation for 31-char identifier");
}

#[test]
fn custom_max_5_six_char_identifier_flagged() {
    let diags = check_with_max("SELECT abcdef FROM t", 5);
    assert_eq!(diags.len(), 1, "abcdef (6 chars) should be flagged with max=5");
    assert!(diags[0].message.contains("abcdef"), "message should contain the identifier name");
}

#[test]
fn custom_max_5_five_char_no_violation() {
    let diags = check_with_max("SELECT abcde FROM t", 5);
    assert!(diags.is_empty(), "abcde (5 chars) should not be flagged with max=5");
}

#[test]
fn quoted_identifier_over_limit_flagged() {
    // Content inside quotes is 34 chars > 30
    let sql = r#"SELECT "this_is_a_very_long_column_name_x" FROM t"#;
    let diags = check(sql);
    assert!(!diags.is_empty(), "quoted identifier exceeding 30 chars should be flagged");
}

#[test]
fn backtick_identifier_over_limit_flagged() {
    // Content inside backticks is 34 chars > 30
    let sql = "SELECT `this_is_a_very_long_column_name_x` FROM t";
    let diags = check(sql);
    assert!(!diags.is_empty(), "backtick identifier exceeding 30 chars should be flagged");
}

#[test]
fn keyword_not_flagged() {
    // SQL keywords like SELECT, FROM, WHERE should not be flagged even if they were >=threshold
    let diags = check("SELECT id FROM t WHERE id = 1");
    assert!(diags.is_empty(), "SQL keywords should not be flagged");
}

#[test]
fn identifier_in_string_not_flagged() {
    // A long identifier inside a string literal should not be flagged
    let sql = "WHERE note = 'averylongidentifierthatexceedsthirtycharshere'";
    let diags = check(sql);
    assert!(diags.is_empty(), "identifiers inside string literals should not be flagged");
}

#[test]
fn message_contains_name_and_length() {
    let ident = "a".repeat(31);
    let sql = format!("SELECT {} FROM t", ident);
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains(&ident), "message should contain the identifier name");
    assert!(diags[0].message.contains("30"), "message should contain the max length");
    assert!(diags[0].message.contains("31"), "message should contain the actual length");
}

#[test]
fn default_max_is_thirty() {
    assert_eq!(MaxIdentifierLength::default().max_length, 30);
}

#[test]
fn line_nonzero() {
    let ident = "a".repeat(31);
    let sql = format!("SELECT {} FROM t", ident);
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line should be 1-indexed");
    assert_eq!(diags[0].line, 1);
}

#[test]
fn col_nonzero() {
    let ident = "a".repeat(31);
    let sql = format!("SELECT {} FROM t", ident);
    // "SELECT " is 7 chars, so identifier starts at col 8
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1, "col should be 1-indexed");
    assert_eq!(diags[0].col, 8);
}

#[test]
fn alias_over_limit_flagged() {
    let sql = "SELECT id AS this_is_a_very_long_alias_name_here FROM t";
    let diags = check(sql);
    assert!(!diags.is_empty(), "alias exceeding 30 chars should be flagged");
    let aliased = diags.iter().any(|d| d.message.contains("this_is_a_very_long_alias_name_here"));
    assert!(aliased, "violation message should mention the long alias");
}
