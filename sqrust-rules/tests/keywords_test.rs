use sqrust_core::FileContext;
use sqrust_rules::capitalisation::keywords::Keywords;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    Keywords.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    let ctx = FileContext::from_source("SELECT 1", "test.sql");
    let diags = Keywords.check(&ctx);
    let _ = diags;
    assert_eq!(Keywords.name(), "Capitalisation/Keywords");
}

#[test]
fn uppercase_keyword_no_violation() {
    assert!(check("SELECT id FROM users").is_empty());
}

#[test]
fn lowercase_select_flagged() {
    let diags = check("select id FROM users");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Capitalisation/Keywords");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn mixed_case_select_flagged() {
    let diags = check("Select id FROM users");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn lowercase_from_flagged() {
    // "SELECT id from users" — "from" starts at byte offset 10 (0-indexed),
    // which is col 11 (1-indexed).
    let diags = check("SELECT id from users");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 11);
}

#[test]
fn multiple_lowercase_keywords_all_flagged() {
    let diags = check("select id from users");
    assert_eq!(diags.len(), 2);
}

#[test]
fn keyword_inside_single_quoted_string_skipped() {
    assert!(check("SELECT 'select' FROM users").is_empty());
}

#[test]
fn keyword_inside_line_comment_skipped() {
    assert!(check("SELECT id FROM users -- select more").is_empty());
}

#[test]
fn keyword_inside_block_comment_skipped() {
    assert!(check("SELECT id /* select from */ FROM users").is_empty());
}

#[test]
fn keyword_inside_double_quoted_identifier_skipped() {
    assert!(check(r#"SELECT "select" FROM users"#).is_empty());
}

#[test]
fn keyword_inside_backtick_identifier_skipped() {
    assert!(check("SELECT `select` FROM users").is_empty());
}

#[test]
fn from_in_from_table_not_flagged() {
    // "from" is part of "from_table" — not a standalone keyword
    assert!(check("SELECT id FROM from_table").is_empty());
}

#[test]
fn select_prefix_in_longer_word_not_flagged() {
    // "selected" must NOT flag "select"
    assert!(check("SELECT selected FROM users").is_empty());
}

#[test]
fn lowercase_keyword_on_second_line_has_correct_line_number() {
    let diags = check("SELECT id\nfrom users");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn lowercase_keyword_mid_line_has_correct_col() {
    // "SELECT 1 WHERE true and false"
    // S(1)E(2)L(3)E(4)C(5)T(6) (7)1(8) (9)W(10)H(11)E(12)R(13)E(14) (15)
    // t(16)r(17)u(18)e(19) (20)a(21)n(22)d(23)... => "and" starts at col 21
    let diags = check("SELECT 1 WHERE true and false");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 21);
}

#[test]
fn keyword_message_format() {
    let diags = check("select id FROM users");
    assert_eq!(
        diags[0].message,
        "Keyword 'select' should be UPPERCASE (use 'SELECT')"
    );
}

#[test]
fn all_uppercase_multiline_no_violations() {
    let sql = "SELECT id, name\nFROM users\nWHERE id > 1\nAND name IS NOT NULL\n";
    assert!(check(sql).is_empty());
}

#[test]
fn escaped_quote_in_string_does_not_break_parsing() {
    // The \'s inside the string should not close the string early
    assert!(check(r"SELECT 'it''s a select' FROM users").is_empty());
}

#[test]
fn block_comment_multiline_keyword_skipped() {
    let sql = "SELECT id\n/* from\n   where */\nFROM users";
    assert!(check(sql).is_empty());
}
