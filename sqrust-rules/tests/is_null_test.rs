use sqrust_core::FileContext;
use sqrust_rules::convention::is_null::IsNull;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    IsNull.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(IsNull.name(), "Convention/IsNull");
}

#[test]
fn eq_null_is_flagged() {
    let diags = check("SELECT * FROM t WHERE col = NULL");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Convention/IsNull");
}

#[test]
fn eq_null_message_is_correct() {
    let diags = check("SELECT * FROM t WHERE col = NULL");
    assert_eq!(diags[0].message, "Use IS NULL instead of = NULL");
}

#[test]
fn eq_null_col_points_to_equals_operator() {
    // "SELECT * FROM t WHERE col = NULL"
    //  1234567890123456789012345678
    // '=' is at position 27 (1-indexed)
    let diags = check("SELECT * FROM t WHERE col = NULL");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 27);
}

#[test]
fn ne_ansi_null_is_flagged() {
    let diags = check("SELECT * FROM t WHERE col <> NULL");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "Use IS NOT NULL instead of <> NULL");
}

#[test]
fn ne_ansi_null_col_points_to_operator() {
    // "SELECT * FROM t WHERE col <> NULL"
    //  1234567890123456789012345678
    // '<' is at position 27 (1-indexed)
    let diags = check("SELECT * FROM t WHERE col <> NULL");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 27);
}

#[test]
fn ne_bang_null_is_flagged() {
    let diags = check("SELECT * FROM t WHERE col != NULL");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "Use IS NOT NULL instead of != NULL");
}

#[test]
fn is_null_no_violation() {
    let diags = check("SELECT * FROM t WHERE col IS NULL");
    assert!(diags.is_empty());
}

#[test]
fn is_not_null_no_violation() {
    let diags = check("SELECT * FROM t WHERE col IS NOT NULL");
    assert!(diags.is_empty());
}

#[test]
fn null_in_string_literal_no_violation() {
    let diags = check("SELECT * FROM t WHERE col = 'NULL'");
    assert!(diags.is_empty());
}

#[test]
fn null_in_line_comment_no_violation() {
    let diags = check("SELECT * FROM t -- WHERE col = NULL");
    assert!(diags.is_empty());
}

#[test]
fn null_in_block_comment_no_violation() {
    let diags = check("SELECT 1 /* col = NULL */");
    assert!(diags.is_empty());
}

#[test]
fn eq_zero_no_violation() {
    let diags = check("SELECT * FROM t WHERE col = 0");
    assert!(diags.is_empty());
}

#[test]
fn multiple_violations_different_lines() {
    let sql = "SELECT *\nFROM t\nWHERE a = NULL\nAND b <> NULL\nAND c != NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 3);
    assert_eq!(diags[0].line, 3);
    assert_eq!(diags[1].line, 4);
    assert_eq!(diags[2].line, 5);
}

#[test]
fn fix_eq_null_replaced_with_is_null() {
    let ctx = FileContext::from_source("SELECT * FROM t WHERE col = NULL", "test.sql");
    let fixed = IsNull.fix(&ctx).expect("fix should be available");
    assert_eq!(fixed, "SELECT * FROM t WHERE col IS NULL");
}

#[test]
fn fix_ne_bang_null_replaced_with_is_not_null() {
    let ctx = FileContext::from_source("SELECT * FROM t WHERE col != NULL", "test.sql");
    let fixed = IsNull.fix(&ctx).expect("fix should be available");
    assert_eq!(fixed, "SELECT * FROM t WHERE col IS NOT NULL");
}

#[test]
fn fix_ne_ansi_null_replaced_with_is_not_null() {
    let ctx = FileContext::from_source("SELECT * FROM t WHERE col <> NULL", "test.sql");
    let fixed = IsNull.fix(&ctx).expect("fix should be available");
    assert_eq!(fixed, "SELECT * FROM t WHERE col IS NOT NULL");
}

#[test]
fn fix_no_change_when_already_correct() {
    let ctx = FileContext::from_source("SELECT * FROM t WHERE col IS NULL", "test.sql");
    // Either None or Some(unchanged) is acceptable — we check None for clean impl
    let result = IsNull.fix(&ctx);
    // If None is returned, no change needed — correct
    // If Some(s) is returned, it must equal the original
    if let Some(fixed) = result {
        assert_eq!(fixed, "SELECT * FROM t WHERE col IS NULL");
    }
}

#[test]
fn fix_does_not_replace_inside_string() {
    let ctx = FileContext::from_source("SELECT * FROM t WHERE x = 'NULL' AND y = NULL", "test.sql");
    let fixed = IsNull.fix(&ctx).expect("fix should be available");
    assert_eq!(fixed, "SELECT * FROM t WHERE x = 'NULL' AND y IS NULL");
}

#[test]
fn null_comparison_lowercase_null_is_flagged() {
    let diags = check("SELECT * FROM t WHERE col = null");
    assert_eq!(diags.len(), 1);
}
