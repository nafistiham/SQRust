use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::null_safe_equality::NullSafeEquality;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    NullSafeEquality.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NullSafeEquality.name(), "Ambiguous/NullSafeEquality");
}

#[test]
fn null_safe_operator_violation() {
    let d = check("SELECT * FROM t WHERE a <=> b");
    assert_eq!(d.len(), 1);
}

#[test]
fn is_not_distinct_from_violation() {
    let d = check("SELECT * FROM t WHERE a IS NOT DISTINCT FROM b");
    assert_eq!(d.len(), 1);
}

#[test]
fn regular_equals_no_violation() {
    assert!(check("SELECT * FROM t WHERE a = b").is_empty());
}

#[test]
fn is_null_no_violation() {
    assert!(check("SELECT * FROM t WHERE a IS NULL").is_empty());
}

#[test]
fn is_distinct_from_violation() {
    let d = check("SELECT * FROM t WHERE a IS DISTINCT FROM b");
    assert_eq!(d.len(), 1);
}

#[test]
fn null_safe_in_string_no_violation() {
    assert!(check("SELECT '<=>' FROM t").is_empty());
}

#[test]
fn null_safe_in_comment_no_violation() {
    assert!(check("-- <=> example\nSELECT 1").is_empty());
}

#[test]
fn multiple_null_safe_multiple_violations() {
    let d = check("SELECT * FROM t WHERE a <=> b AND c <=> d");
    assert_eq!(d.len(), 2);
}

#[test]
fn null_safe_message_content() {
    let d = check("SELECT * FROM t WHERE a <=> b");
    let msg = d[0].message.to_uppercase();
    assert!(msg.contains("<=>") || msg.contains("NULL-SAFE") || msg.contains("MYSQL"));
}

#[test]
fn is_not_distinct_message_content() {
    let d = check("SELECT * FROM t WHERE a IS NOT DISTINCT FROM b");
    let msg = d[0].message.to_uppercase();
    assert!(
        msg.contains("DISTINCT")
            || msg.contains("INCONSISTENT")
            || msg.contains("SUPPORT")
    );
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT * FROM t WHERE a <=> b");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn greater_than_or_equal_no_violation() {
    // >= is not <=>
    assert!(check("SELECT * FROM t WHERE a >= b").is_empty());
}
