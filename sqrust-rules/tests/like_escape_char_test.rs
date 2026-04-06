use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::like_escape_char::LikeEscapeChar;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    LikeEscapeChar.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(LikeEscapeChar.name(), "Ambiguous/LikeEscapeChar");
}

#[test]
fn backslash_underscore_violation() {
    let d = check("SELECT * FROM t WHERE name LIKE '100\\_foo'");
    assert_eq!(d.len(), 1);
}

#[test]
fn backslash_percent_violation() {
    let d = check("SELECT * FROM t WHERE name LIKE '50\\%off'");
    assert_eq!(d.len(), 1);
}

#[test]
fn with_escape_clause_no_violation() {
    // The ESCAPE clause makes it explicit, so no violation.
    let d = check("SELECT * FROM t WHERE name LIKE '100\\_foo' ESCAPE '\\'");
    assert_eq!(d.len(), 0);
}

#[test]
fn normal_wildcard_no_violation() {
    // Plain wildcards — no backslash escaping, no violation.
    let d = check("SELECT * FROM t WHERE name LIKE '%foo%'");
    assert_eq!(d.len(), 0);
}

#[test]
fn no_like_no_violation() {
    let d = check("SELECT * FROM t WHERE a = 1");
    assert_eq!(d.len(), 0);
}

#[test]
fn like_in_comment_no_violation() {
    // The LIKE with backslash is inside a comment — should not flag.
    let d = check("-- LIKE '100\\_foo'\nSELECT 1");
    assert_eq!(d.len(), 0);
}

#[test]
fn not_like_with_backslash_violation() {
    let d = check("SELECT * FROM t WHERE name NOT LIKE '50\\_off'");
    assert_eq!(d.len(), 1);
}

#[test]
fn multiple_violations() {
    let d = check(
        "SELECT * FROM t WHERE a LIKE '10\\%x' AND b LIKE 'y\\_z'"
    );
    assert_eq!(d.len(), 2);
}

#[test]
fn like_without_backslash_no_violation() {
    // Naked _ is a wildcard — we don't flag it (no backslash escape attempt).
    let d = check("SELECT * FROM t WHERE name LIKE 'foo_bar'");
    assert_eq!(d.len(), 0);
}

#[test]
fn backslash_in_non_like_string_no_violation() {
    // Backslash escape in a string that is NOT a LIKE pattern — no violation.
    let d = check("SELECT '100\\_foo' FROM t");
    assert_eq!(d.len(), 0);
}

#[test]
fn ilike_with_backslash_violation() {
    // ILIKE uses the same wildcard semantics; backslash escape is also ambiguous.
    let d = check("SELECT * FROM t WHERE name ILIKE '50\\_off'");
    assert_eq!(d.len(), 1);
}

#[test]
fn empty_file_no_violation() {
    let d = check("");
    assert_eq!(d.len(), 0);
}
