use sqrust_core::FileContext;
use sqrust_rules::convention::no_ilike::NoIlike;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoIlike.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoIlike.name(), "Convention/NoIlike");
}

#[test]
fn ilike_simple_violation() {
    let diags = check("SELECT * FROM t WHERE name ILIKE '%foo%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn ilike_lowercase_violation() {
    let diags = check("SELECT * FROM t WHERE name ilike '%foo%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn ilike_mixed_case_violation() {
    let diags = check("SELECT * FROM t WHERE name Ilike '%foo%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_ilike_violation() {
    // NOT ILIKE still contains the ILIKE keyword — should be flagged
    let diags = check("SELECT * FROM t WHERE name NOT ILIKE '%foo%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn ilike_in_string_no_violation() {
    let diags = check("SELECT 'WHERE name ILIKE foo' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn ilike_in_comment_no_violation() {
    let diags = check("-- ILIKE\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn like_no_violation() {
    let diags = check("SELECT * FROM t WHERE name LIKE '%foo%'");
    assert!(diags.is_empty());
}

#[test]
fn ilike_word_boundary_no_violation() {
    // "ilike_score" is an identifier — ILIKE must have a word boundary after it
    let diags = check("SELECT ilike_score FROM t");
    assert!(diags.is_empty());
}

#[test]
fn multiple_ilike_two_violations() {
    let diags = check(
        "SELECT * FROM t WHERE name ILIKE '%foo%' OR name ILIKE '%bar%'",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn ilike_uppercase_violation() {
    let diags = check("SELECT * FROM t WHERE col ILIKE 'ABC%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_content_check() {
    let diags = check("SELECT * FROM t WHERE name ILIKE '%x%'");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    assert!(
        msg.contains("LOWER"),
        "message should contain 'LOWER', got: {msg}"
    );
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}
