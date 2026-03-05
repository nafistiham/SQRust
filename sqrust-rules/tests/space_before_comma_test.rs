use sqrust_core::FileContext;
use sqrust_rules::layout::space_before_comma::SpaceBeforeComma;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SpaceBeforeComma.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SpaceBeforeComma.name(), "Layout/SpaceBeforeComma");
}

#[test]
fn space_before_comma_single_produces_one_violation() {
    let diags = check("SELECT col1 , col2");
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_space_before_comma_has_no_violation() {
    let diags = check("SELECT col1, col2");
    assert!(diags.is_empty());
}

#[test]
fn multiple_spaces_before_comma_produces_one_violation() {
    let diags = check("SELECT col1  , col2");
    assert_eq!(diags.len(), 1);
}

#[test]
fn leading_comma_style_no_space_on_own_line_has_no_violation() {
    // comma at start of line — leading-comma style, not a violation
    let diags = check("SELECT\n  col1\n  ,col2");
    assert!(diags.is_empty());
}

#[test]
fn leading_comma_with_space_after_on_own_line_has_no_violation() {
    // comma at start of line with a space after — still leading-comma style
    let diags = check("SELECT\n  col1\n  , col2");
    assert!(diags.is_empty());
}

#[test]
fn two_space_before_comma_violations_produces_two_violations() {
    let diags = check("SELECT col1 , col2 , col3");
    assert_eq!(diags.len(), 2);
}

#[test]
fn comma_in_single_quoted_string_has_no_violation() {
    let diags = check("SELECT 'a , b'");
    assert!(diags.is_empty());
}

#[test]
fn comma_in_line_comment_has_no_violation() {
    let diags = check("-- col1 ,");
    assert!(diags.is_empty());
}

#[test]
fn comma_in_block_comment_has_no_violation() {
    let diags = check("/* col1 , col2 */");
    assert!(diags.is_empty());
}

#[test]
fn fix_removes_space_before_comma() {
    let ctx = FileContext::from_source("col1 , col2", "test.sql");
    let fixed = SpaceBeforeComma.fix(&ctx).expect("fix should return Some");
    assert_eq!(fixed, "col1, col2");
}

#[test]
fn correct_message_text() {
    let diags = check("SELECT col1 , col2");
    assert_eq!(diags[0].message, "Remove space before comma");
}

#[test]
fn tab_before_comma_produces_one_violation() {
    let diags = check("SELECT col1\t, col2");
    assert_eq!(diags.len(), 1);
}

#[test]
fn col_points_to_first_space_before_comma() {
    // "SELECT col1 , col2"
    //  123456789012345
    //            ^ col 12 is the space before the comma
    let diags = check("SELECT col1 , col2");
    assert_eq!(diags[0].col, 12);
}
