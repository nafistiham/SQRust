use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::leading_zero_numeric::LeadingZeroNumeric;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    LeadingZeroNumeric.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(LeadingZeroNumeric.name(), "Convention/LeadingZeroNumeric");
}

#[test]
fn no_decimal_no_violation() {
    let diags = check("SELECT 5 + 3");
    assert!(diags.is_empty());
}

#[test]
fn proper_decimal_no_violation() {
    let diags = check("SELECT 0.5");
    assert!(diags.is_empty());
}

#[test]
fn leading_dot_in_where_flagged() {
    let diags = check("SELECT id FROM t WHERE price > .5");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Convention/LeadingZeroNumeric");
}

#[test]
fn leading_dot_after_equals_flagged() {
    let diags = check("SELECT id FROM t WHERE ratio = .25");
    assert_eq!(diags.len(), 1);
}

#[test]
fn leading_dot_after_open_paren_flagged() {
    let diags = check("SELECT id FROM t WHERE (ratio > .1)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn leading_dot_after_comma_flagged() {
    let diags = check("SELECT .5, .25 FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn leading_dot_in_string_not_flagged() {
    let diags = check("SELECT id FROM t WHERE note = '.5 miles'");
    assert!(diags.is_empty());
}

#[test]
fn leading_dot_in_comment_not_flagged() {
    let diags = check("-- .5 is the value\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn table_dot_column_not_flagged() {
    // t.id — the '.' follows a letter, not a trigger character
    let diags = check("SELECT t.id FROM t");
    assert!(diags.is_empty());
}

#[test]
fn digit_dot_not_flagged() {
    // 1.5 — the '.' follows a digit
    let diags = check("SELECT 1.5 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn violation_message_contains_zero() {
    let diags = check("SELECT .5 FROM t");
    let msg = &diags[0].message;
    let has_leading_zero = msg.contains("leading zero") || msg.contains("0.");
    assert!(
        has_leading_zero,
        "message should mention 'leading zero' or '0.', got: {msg}"
    );
}

#[test]
fn violation_line_nonzero() {
    let diags = check("SELECT .5 FROM t");
    assert!(diags[0].line >= 1);
}

#[test]
fn violation_col_nonzero() {
    let diags = check("SELECT .5 FROM t");
    assert!(diags[0].col >= 1);
}

#[test]
fn two_violations_counted() {
    let diags = check("SELECT .5 + .25 FROM t");
    assert_eq!(diags.len(), 2);
}
