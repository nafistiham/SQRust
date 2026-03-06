use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::mixed_line_endings::MixedLineEndings;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    MixedLineEndings.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    // Need a source with mixed endings to get a violation and inspect the name
    let sql = "SELECT 1\r\nFROM t\nWHERE 1=1";
    let diags = check(sql);
    assert!(!diags.is_empty());
    assert_eq!(diags[0].rule, "Layout/MixedLineEndings");
}

#[test]
fn parse_error_produces_no_violations() {
    let ctx = FileContext::from_source("SELECT FROM FROM", "test.sql");
    assert!(!ctx.parse_errors.is_empty());
    let diags = MixedLineEndings.check(&ctx);
    assert!(diags.is_empty());
}

#[test]
fn only_lf_no_violations() {
    let diags = check("SELECT 1\nFROM t\nWHERE 1=1");
    assert!(diags.is_empty());
}

#[test]
fn only_crlf_no_violations() {
    let diags = check("SELECT 1\r\nFROM t\r\nWHERE 1=1");
    assert!(diags.is_empty());
}

#[test]
fn mixed_crlf_and_lf_one_violation() {
    let diags = check("SELECT 1\r\nFROM t\nWHERE 1=1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn single_line_no_newlines_no_violations() {
    let diags = check("SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn multi_line_all_lf_no_violations() {
    let diags = check("SELECT a,\nb,\nc\nFROM t");
    assert!(diags.is_empty());
}

#[test]
fn multi_line_all_crlf_no_violations() {
    let diags = check("SELECT a,\r\nb,\r\nc\r\nFROM t");
    assert!(diags.is_empty());
}

#[test]
fn one_crlf_then_one_lf_one_violation() {
    let diags = check("SELECT 1\r\nFROM t\n");
    assert_eq!(diags.len(), 1);
}

#[test]
fn violation_is_at_line_1_col_1() {
    let diags = check("SELECT 1\r\nFROM t\nWHERE 1=1");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn only_one_violation_even_with_many_mixed_lines() {
    // Alternate CRLF and LF across many lines — still only 1 violation.
    // Uses valid SQL so the parse guard does not fire.
    let sql = "SELECT 1\r\nFROM t\nWHERE 1=1\r\nAND 2=2\nAND 3=3\r\n";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_format_correct() {
    let diags = check("SELECT 1\r\nFROM t\nWHERE 1=1");
    assert!(!diags.is_empty());
    assert_eq!(
        diags[0].message,
        "Mixed line endings detected (both CRLF and LF); normalize to one style"
    );
}

#[test]
fn fix_converts_crlf_to_lf() {
    let ctx = FileContext::from_source("SELECT 1\r\nFROM t\r\nWHERE 1=1", "test.sql");
    // Pure CRLF — no mixing — fix not needed, returns None
    let result = MixedLineEndings.fix(&ctx);
    assert!(result.is_none());
}

#[test]
fn fix_normalizes_mixed_to_lf() {
    let ctx = FileContext::from_source("SELECT 1\r\nFROM t\nWHERE 1=1", "test.sql");
    let fixed = MixedLineEndings.fix(&ctx).expect("fix should return Some for mixed endings");
    assert_eq!(fixed, "SELECT 1\nFROM t\nWHERE 1=1");
}

#[test]
fn fix_returns_none_for_pure_lf() {
    let ctx = FileContext::from_source("SELECT 1\nFROM t\n", "test.sql");
    let result = MixedLineEndings.fix(&ctx);
    assert!(result.is_none());
}
