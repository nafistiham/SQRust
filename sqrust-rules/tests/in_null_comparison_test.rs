use sqrust_core::FileContext;
use sqrust_rules::convention::in_null_comparison::InNullComparison;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    InNullComparison.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(InNullComparison.name(), "Convention/InNullComparison");
}

#[test]
fn in_null_is_flagged() {
    let diags = check("SELECT * FROM t WHERE col IN (NULL)");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Convention/InNullComparison");
}

#[test]
fn in_null_message_is_correct() {
    let diags = check("SELECT * FROM t WHERE col IN (NULL)");
    assert_eq!(diags[0].message, "Use IS NULL instead of IN (NULL)");
}

#[test]
fn not_in_null_is_flagged() {
    let diags = check("SELECT * FROM t WHERE col NOT IN (NULL)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_in_null_message_is_correct() {
    let diags = check("SELECT * FROM t WHERE col NOT IN (NULL)");
    assert_eq!(diags[0].message, "Use IS NOT NULL instead of NOT IN (NULL)");
}

#[test]
fn in_value_list_is_clean() {
    let diags = check("SELECT * FROM t WHERE col IN (1, 2, 3)");
    assert!(diags.is_empty());
}

#[test]
fn in_null_with_other_values_is_clean() {
    let diags = check("SELECT * FROM t WHERE col IN (NULL, 1)");
    assert!(diags.is_empty());
}

#[test]
fn is_null_is_clean() {
    let diags = check("SELECT * FROM t WHERE col IS NULL");
    assert!(diags.is_empty());
}

#[test]
fn is_not_null_is_clean() {
    let diags = check("SELECT * FROM t WHERE col IS NOT NULL");
    assert!(diags.is_empty());
}

#[test]
fn in_null_with_inner_whitespace_is_flagged() {
    let diags = check("SELECT * FROM t WHERE col IN ( NULL )");
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_in_null_is_flagged() {
    let diags = check("where col in (null)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn in_null_in_comment_is_ignored() {
    let diags = check("-- WHERE col IN (NULL)\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn in_null_in_string_is_ignored() {
    let diags = check("SELECT * FROM t WHERE col = 'IN (NULL)'");
    assert!(diags.is_empty());
}

#[test]
fn not_in_null_with_whitespace_is_flagged() {
    let diags = check("SELECT * FROM t WHERE col NOT IN ( NULL )");
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_in_null_message_format_for_not_in_null() {
    let diags = check("SELECT * FROM t WHERE col NOT IN (NULL)");
    assert_eq!(diags[0].message, "Use IS NOT NULL instead of NOT IN (NULL)");
}
