use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::nulls_ordering::NullsOrdering;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NullsOrdering.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NullsOrdering.name(), "Ambiguous/NullsOrdering");
}

#[test]
fn parse_error_still_scans_source() {
    // Text-based rule — parse errors do not stop it.
    let ctx = FileContext::from_source("SELECTT GARBAGE @@ ORDER BY x", "test.sql");
    let diags = NullsOrdering.check(&ctx);
    assert_eq!(diags.len(), 1);
}

#[test]
fn order_by_without_nulls_one_violation() {
    let diags = check("SELECT * FROM t ORDER BY name");
    assert_eq!(diags.len(), 1);
}

#[test]
fn order_by_with_nulls_first_no_violation() {
    let diags = check("SELECT * FROM t ORDER BY name NULLS FIRST");
    assert!(diags.is_empty());
}

#[test]
fn order_by_with_nulls_last_no_violation() {
    let diags = check("SELECT * FROM t ORDER BY name NULLS LAST");
    assert!(diags.is_empty());
}

#[test]
fn no_order_by_no_violation() {
    let diags = check("SELECT * FROM t WHERE id = 1");
    assert!(diags.is_empty());
}

#[test]
fn two_order_by_both_missing_nulls_two_violations() {
    let sql = "SELECT * FROM t ORDER BY a; SELECT * FROM t ORDER BY b";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn two_order_by_one_with_nulls_one_without_one_violation() {
    let sql = "SELECT * FROM t ORDER BY a NULLS LAST; SELECT * FROM t ORDER BY b";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn order_by_multiple_columns_without_nulls_one_violation() {
    // ORDER BY a, b is a single ORDER BY clause — flagged once
    let diags = check("SELECT * FROM t ORDER BY a, b, c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_nulls_and_order() {
    let diags = check("SELECT * FROM t ORDER BY name");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.to_ascii_uppercase().contains("ORDER"),
        "message should mention ORDER, got: {msg}"
    );
    assert!(
        msg.to_ascii_uppercase().contains("NULLS"),
        "message should mention NULLS, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM t ORDER BY name");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn col_points_to_order_keyword() {
    // "SELECT * FROM t ORDER BY name"
    //  0         1         2
    //  0123456789012345678901234567890
    // "SELECT * FROM t " is 16 chars, so ORDER starts at col 17
    let sql = "SELECT * FROM t ORDER BY name";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 17);
}

#[test]
fn order_by_in_string_no_violation() {
    // 'ORDER BY x' inside a string literal — must not be flagged
    let diags = check("SELECT 'ORDER BY x' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nulls_last_case_insensitive_no_violation() {
    // lowercase 'nulls last' should still satisfy the rule
    let diags = check("SELECT * FROM t ORDER BY name nulls last");
    assert!(diags.is_empty());
}
