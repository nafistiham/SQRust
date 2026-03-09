use sqrust_core::{FileContext, Rule};
use sqrust_rules::lint::consecutive_semicolons::ConsecutiveSemicolons;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    ConsecutiveSemicolons.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ConsecutiveSemicolons.name(), "Lint/ConsecutiveSemicolons");
}

#[test]
fn single_semicolon_no_violation() {
    assert!(check("SELECT 1;").is_empty());
}

#[test]
fn two_statements_no_violation() {
    assert!(check("SELECT 1;\nSELECT 2;").is_empty());
}

#[test]
fn double_semicolon_flagged() {
    let d = check("SELECT 1;;");
    assert_eq!(d.len(), 1);
}

#[test]
fn triple_semicolon_flagged_once() {
    let d = check("SELECT 1;;;");
    assert_eq!(d.len(), 1);
}

#[test]
fn double_semicolon_on_own_line_flagged() {
    let d = check("SELECT 1;\n;");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_separate_double_semicolons_flagged_twice() {
    let d = check("SELECT 1;;\nSELECT 2;;");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_semicolons() {
    let d = check("SELECT 1;;");
    assert!(d[0].message.contains(';') || d[0].message.to_lowercase().contains("semicolon"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT 1;;");
    assert_eq!(d[0].rule, "Lint/ConsecutiveSemicolons");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT 1;;");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn semicolon_in_string_not_flagged() {
    assert!(check("SELECT ';;' FROM t").is_empty());
}

#[test]
fn semicolon_in_comment_not_flagged() {
    assert!(check("SELECT 1 -- ;;\n;").is_empty());
}

#[test]
fn no_semicolon_no_violation() {
    assert!(check("SELECT 1").is_empty());
}
