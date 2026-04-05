use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::prefer_ansi_trim::PreferAnsiTrim;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    PreferAnsiTrim.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(PreferAnsiTrim.name(), "Convention/PreferAnsiTrim");
}

#[test]
fn ltrim_violation() {
    let diags = check("SELECT LTRIM(name) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn rtrim_violation() {
    let diags = check("SELECT RTRIM(name) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn both_ltrim_rtrim_two_violations() {
    let diags = check("SELECT LTRIM(RTRIM(name)) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn ltrim_lowercase_violation() {
    let diags = check("SELECT ltrim(name) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn rtrim_lowercase_violation() {
    let diags = check("SELECT rtrim(name) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn ltrim_in_string_no_violation() {
    let diags = check("SELECT 'LTRIM(name)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn rtrim_in_comment_no_violation() {
    let diags = check("-- RTRIM(name)\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn trim_function_no_violation() {
    let diags = check("SELECT TRIM(name) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn ltrim_in_column_name_no_violation() {
    // ltrim_value is one word — no word boundary before '(' since '(' never comes
    let diags = check("SELECT ltrim_value FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_trim_function_no_violation() {
    let diags = check("SELECT name FROM t");
    assert!(diags.is_empty());
}

#[test]
fn ltrim_message_content() {
    let diags = check("SELECT LTRIM(name) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("LTRIM"));
}

#[test]
fn rtrim_message_content() {
    let diags = check("SELECT RTRIM(name) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("RTRIM"));
}
