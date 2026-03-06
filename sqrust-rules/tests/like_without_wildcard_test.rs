use sqrust_core::FileContext;
use sqrust_rules::convention::like_without_wildcard::LikeWithoutWildcard;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    LikeWithoutWildcard.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(LikeWithoutWildcard.name(), "Convention/LikeWithoutWildcard");
}

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn like_with_percent_no_violation() {
    let diags = check("SELECT * FROM t WHERE name LIKE '%foo%'");
    assert!(diags.is_empty());
}

#[test]
fn like_without_wildcard_one_violation() {
    let diags = check("SELECT * FROM t WHERE name LIKE 'foo'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn like_with_underscore_no_violation() {
    let diags = check("SELECT * FROM t WHERE name LIKE 'f_o'");
    assert!(diags.is_empty());
}

#[test]
fn like_with_only_underscore_no_violation() {
    let diags = check("SELECT * FROM t WHERE name LIKE '_'");
    assert!(diags.is_empty());
}

#[test]
fn not_like_without_wildcard_one_violation() {
    let diags = check("SELECT * FROM t WHERE name NOT LIKE 'foo'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn ilike_without_wildcard_one_violation() {
    // ILIKE is case-insensitive LIKE — still flag if no wildcard
    let diags = check("SELECT * FROM t WHERE name ILIKE 'foo'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn like_with_column_pattern_no_violation() {
    // pattern is a column reference, not a literal — no violation
    let diags = check("SELECT * FROM t WHERE name LIKE col");
    assert!(diags.is_empty());
}

#[test]
fn no_like_no_violation() {
    let diags = check("SELECT * FROM t WHERE name = 'foo'");
    assert!(diags.is_empty());
}

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT * FROM t WHERE name LIKE 'bar'");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("wildcard") || diags[0].message.contains("="),
        "message was: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM t WHERE name LIKE 'baz'");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn two_like_without_wildcard_two_violations() {
    let diags = check(
        "SELECT * FROM t WHERE name LIKE 'foo' AND email LIKE 'bar'",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn like_with_percent_at_end_no_violation() {
    let diags = check("SELECT * FROM t WHERE name LIKE 'foo%'");
    assert!(diags.is_empty());
}

#[test]
fn like_points_to_like_keyword() {
    // "SELECT * FROM t WHERE name LIKE 'foo'"
    //  1234567890123456789012345678
    //                             ^col 28 = L of LIKE
    let diags = check("SELECT * FROM t WHERE name LIKE 'foo'");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 28);
}
