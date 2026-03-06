use sqrust_core::FileContext;
use sqrust_rules::layout::leading_comma::LeadingComma;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    LeadingComma.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    let ctx = FileContext::from_source("SELECT 1", "test.sql");
    let diags = LeadingComma.check(&ctx);
    // no violations; test rule name via a violation
    let ctx2 = FileContext::from_source("SELECT id\n     , name\nFROM t", "test.sql");
    let diags2 = LeadingComma.check(&ctx2);
    assert_eq!(diags2[0].rule, "Layout/LeadingComma");
    let _ = diags;
}

#[test]
fn leading_comma_one_violation() {
    // Comma at start of line (after whitespace) — one violation
    let sql = "SELECT id\n     , name\nFROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn trailing_comma_no_violation() {
    // Comma at end of line — trailing comma style — no violation
    let sql = "SELECT id,\n       name\nFROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn single_line_no_violation() {
    // All on one line — no leading comma
    let sql = "SELECT id, name FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_leading_commas_two_violations() {
    // Three-column SELECT with two leading commas
    let sql = "SELECT id\n     , name\n     , email\nFROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn leading_comma_with_whitespace_violation() {
    // Spaces before comma — still a leading comma violation
    let sql = "SELECT id\n   , name\nFROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_comma_no_violation() {
    let sql = "SELECT id FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn comma_mid_line_no_violation() {
    // Comma in the middle of a line — not a leading comma
    let sql = "SELECT id, name,\n       email\nFROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn comma_inside_string_no_violation() {
    // Comma inside a string literal — should not flag
    let sql = "SELECT 'a,b' FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_contains_useful_text() {
    let sql = "SELECT id\n     , name\nFROM t";
    let diags = check(sql);
    assert!(!diags[0].message.is_empty());
    // Message should mention commas or placement
    let msg_lower = diags[0].message.to_lowercase();
    assert!(
        msg_lower.contains("comma") || msg_lower.contains("start"),
        "Expected message about comma placement, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "SELECT id\n     , name\nFROM t";
    let diags = check(sql);
    assert!(diags[0].line > 0);
    assert!(diags[0].col > 0);
}

#[test]
fn col_points_to_comma() {
    // "     , name" — 5 spaces then comma. col should be 6 (1-indexed).
    let sql = "SELECT id\n     , name\nFROM t";
    let diags = check(sql);
    assert_eq!(diags[0].line, 2);
    assert_eq!(diags[0].col, 6);
}

#[test]
fn parse_error_still_checks_source() {
    // Even with parse errors (invalid SQL), the text-based scan should still run
    let sql = "SELECT id\n     , name\nFROM @@invalid@@";
    let ctx = FileContext::from_source(sql, "test.sql");
    // Confirm parse error exists
    let diags = LeadingComma.check(&ctx);
    // Should still find the leading comma violation
    assert_eq!(diags.len(), 1);
}

#[test]
fn empty_source_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}
