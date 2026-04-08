use sqrust_core::FileContext;
use sqrust_rules::convention::no_similar_to::NoSimilarTo;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoSimilarTo.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoSimilarTo.name(), "Convention/NoSimilarTo");
}

#[test]
fn similar_to_violation() {
    let diags = check("SELECT * FROM t WHERE name SIMILAR TO '%foo%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn like_no_violation() {
    let diags = check("SELECT * FROM t WHERE name LIKE '%foo%'");
    assert!(diags.is_empty());
}

#[test]
fn similar_to_in_string_no_violation() {
    let diags = check("SELECT 'WHERE name SIMILAR TO foo' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn similar_to_in_comment_no_violation() {
    let diags = check("-- SIMILAR TO\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn similar_to_in_block_comment_no_violation() {
    let diags = check("/* SIMILAR TO pattern */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn similar_to_case_insensitive() {
    let diags_upper = check("SELECT * FROM t WHERE col SIMILAR TO 'abc'");
    assert_eq!(diags_upper.len(), 1, "SIMILAR TO (upper) should trigger");

    let diags_lower = check("SELECT * FROM t WHERE col similar to 'abc'");
    assert_eq!(diags_lower.len(), 1, "similar to (lower) should trigger");

    let diags_mixed = check("SELECT * FROM t WHERE col Similar To 'abc'");
    assert_eq!(diags_mixed.len(), 1, "Similar To (mixed) should trigger");
}

#[test]
fn not_similar_to_violation() {
    // NOT SIMILAR TO still contains the SIMILAR TO keywords — should be flagged
    let diags = check("SELECT * FROM t WHERE name NOT SIMILAR TO '%foo%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn multi_line_violation() {
    let sql = "SELECT *\nFROM t\nWHERE col SIMILAR TO 'pattern'";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}

#[test]
fn select_only_no_violation() {
    let diags = check("SELECT id, name FROM users");
    assert!(diags.is_empty());
}

#[test]
fn similar_to_partial_word_no_violation() {
    // "SIMILAR_TO_SOMETHING" is a single identifier — should NOT trigger
    let diags = check("SELECT SIMILAR_TO_SOMETHING FROM t");
    assert!(diags.is_empty());
}

#[test]
fn regex_operator_no_violation() {
    // col ~ 'pattern' is the regex operator — should not trigger
    let diags = check("SELECT * FROM t WHERE col ~ 'pattern'");
    assert!(diags.is_empty());
}
