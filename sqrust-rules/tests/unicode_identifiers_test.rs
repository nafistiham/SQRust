use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::unicode_identifiers::UnicodeIdentifiers;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    UnicodeIdentifiers.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    let ctx = FileContext::from_source("SELECT 1", "test.sql");
    let diags = UnicodeIdentifiers.check(&ctx);
    // No violations in pure ASCII, but the name is wired correctly — verify via a
    // source that does produce a violation.
    let ctx2 = FileContext::from_source("SELECT * FROM café", "test.sql");
    let diags2 = UnicodeIdentifiers.check(&ctx2);
    assert!(!diags2.is_empty());
    assert_eq!(diags2[0].rule, "Layout/UnicodeIdentifiers");
    let _ = diags; // silence unused
}

#[test]
fn parse_error_produces_no_violations() {
    // A source that fails to parse should yield no violations (parse_errors is non-empty)
    let ctx = FileContext::from_source("SELECT FROM FROM", "test.sql");
    assert!(!ctx.parse_errors.is_empty());
    let diags = UnicodeIdentifiers.check(&ctx);
    assert!(diags.is_empty());
}

#[test]
fn pure_ascii_sql_no_violations() {
    let diags = check("SELECT id, name FROM users WHERE id = 1;");
    assert!(diags.is_empty());
}

#[test]
fn non_ascii_in_unquoted_table_name_flagged() {
    // "café" has non-ASCII characters starting at the 'é'
    let diags = check("SELECT * FROM café");
    assert!(!diags.is_empty());
}

#[test]
fn non_ascii_in_unquoted_column_name_flagged() {
    let diags = check("SELECT prénom FROM t");
    assert!(!diags.is_empty());
}

#[test]
fn non_ascii_inside_single_quoted_string_not_flagged() {
    let diags = check("SELECT * FROM t WHERE name = 'André'");
    assert!(diags.is_empty());
}

#[test]
fn non_ascii_inside_double_quoted_identifier_not_flagged() {
    let diags = check("SELECT \"prénom\" FROM t");
    assert!(diags.is_empty());
}

#[test]
fn non_ascii_inside_block_comment_not_flagged() {
    let diags = check("/* ünde */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn non_ascii_inside_line_comment_not_flagged() {
    let diags = check("SELECT 1 -- ünde\nFROM t");
    assert!(diags.is_empty());
}

#[test]
fn multiple_non_ascii_chars_in_one_word_one_violation_per_char() {
    // "café" has 2 non-ASCII bytes in 'é' (2-byte UTF-8), but 1 character.
    // "naïve" has 'ï' — 1 non-ASCII character.
    // We flag each non-ASCII CHARACTER (not byte) — so "ée" in "ée" = 2 chars.
    let diags = check("SELECT éà FROM t");
    // 'é' is 1 non-ASCII char, 'à' is 1 non-ASCII char => 2 violations
    assert_eq!(diags.len(), 2);
}

#[test]
fn non_ascii_in_both_quoted_and_unquoted_only_unquoted_flagged() {
    // 'André' in single quotes = 0; café unquoted = non-ASCII chars flagged
    let diags = check("SELECT * FROM café WHERE name = 'André'");
    // 'é' in café is flagged; André in single quotes is not
    assert!(!diags.is_empty());
    // All violations must come from "café" (unquoted), not from 'André' (quoted)
    for d in &diags {
        // 'André' is on the same line — violations must be within "café" range.
        // café starts at col 15: S-E-L-E-C-T- -*-space-F-R-O-M-space = col 14, 'c' = col 15
        // 'é' is multi-byte but col should point to position > 15 and < WHERE position
        assert!(d.col < 22, "col {} should be within 'café'", d.col);
    }
}

#[test]
fn line_and_col_reported_correctly() {
    // "SELECT 1\nFROM café" — café is on line 2
    let diags = check("SELECT 1\nFROM café");
    assert!(!diags.is_empty());
    assert_eq!(diags[0].line, 2);
    // "FROM " is 5 chars, 'c' is col 6, 'a' col 7, 'f' col 8, 'é' is col 9
    assert_eq!(diags[0].col, 9);
}

#[test]
fn message_format_correct() {
    let diags = check("SELECT * FROM café");
    assert!(!diags.is_empty());
    assert_eq!(
        diags[0].message,
        "Non-ASCII character found in SQL; use ASCII identifiers for portability"
    );
}

#[test]
fn empty_sql_no_violations() {
    let diags = check("");
    assert!(diags.is_empty());
}
