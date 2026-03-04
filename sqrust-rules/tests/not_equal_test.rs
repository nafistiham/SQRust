use sqrust_core::FileContext;
use sqrust_rules::convention::not_equal::NotEqual;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NotEqual.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NotEqual.name(), "Convention/NotEqual");
}

#[test]
fn not_equal_operator_is_flagged() {
    let diags = check("SELECT * FROM t WHERE a != b");
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_equal_violation_has_correct_line_and_col() {
    // "SELECT * FROM t WHERE a != b"
    //  123456789012345678901234567
    // '!' is at col 25
    let diags = check("SELECT * FROM t WHERE a != b");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 25);
}

#[test]
fn not_equal_violation_has_correct_message() {
    let diags = check("SELECT * FROM t WHERE a != b");
    assert_eq!(
        diags[0].message,
        "Use '<>' instead of '!=' for ANSI SQL compatibility"
    );
}

#[test]
fn ansi_not_equal_operator_has_no_violation() {
    let diags = check("SELECT * FROM t WHERE a <> b");
    assert!(diags.is_empty());
}

#[test]
fn not_equal_inside_single_quoted_string_skipped() {
    let diags = check("SELECT * FROM t WHERE x = '!='");
    assert!(diags.is_empty());
}

#[test]
fn not_equal_inside_double_quoted_identifier_skipped() {
    let diags = check(r#"SELECT "col!=" FROM t"#);
    assert!(diags.is_empty());
}

#[test]
fn not_equal_inside_line_comment_skipped() {
    let diags = check("SELECT 1 -- WHERE a != b");
    assert!(diags.is_empty());
}

#[test]
fn not_equal_inside_block_comment_skipped() {
    let diags = check("SELECT 1 /* WHERE a != b */");
    assert!(diags.is_empty());
}

#[test]
fn multiple_not_equal_on_different_lines_all_flagged() {
    let sql = "SELECT *\nFROM t\nWHERE a != b\nAND c != d";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
    assert_eq!(diags[0].line, 3);
    assert_eq!(diags[1].line, 4);
}

#[test]
fn not_equal_on_second_line_correct_line_number() {
    let sql = "SELECT 1\nFROM t WHERE a != b";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn fix_replaces_not_equal_with_ansi() {
    let ctx = FileContext::from_source("SELECT * FROM t WHERE a != b", "test.sql");
    let fixed = NotEqual.fix(&ctx).expect("fix should be available");
    assert_eq!(fixed, "SELECT * FROM t WHERE a <> b");
}

#[test]
fn fix_replaces_multiple_not_equal_occurrences() {
    let ctx = FileContext::from_source("WHERE a != b AND c != d", "test.sql");
    let fixed = NotEqual.fix(&ctx).expect("fix should be available");
    assert_eq!(fixed, "WHERE a <> b AND c <> d");
}

#[test]
fn fix_does_not_replace_inside_string() {
    let ctx = FileContext::from_source("SELECT '!=' FROM t WHERE a != b", "test.sql");
    let fixed = NotEqual.fix(&ctx).expect("fix should be available");
    assert_eq!(fixed, "SELECT '!=' FROM t WHERE a <> b");
}
