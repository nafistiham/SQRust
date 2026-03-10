use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::floating_point_comparison::FloatingPointComparison;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    FloatingPointComparison.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(FloatingPointComparison.name(), "Ambiguous/FloatingPointComparison");
}

#[test]
fn integer_comparison_no_violation() {
    assert!(check("SELECT id FROM t WHERE price = 10").is_empty());
}

#[test]
fn string_comparison_no_violation() {
    assert!(check("SELECT id FROM t WHERE name = 'hello'").is_empty());
}

#[test]
fn greater_than_float_no_violation() {
    // > and < are fine — not exact equality
    assert!(check("SELECT id FROM t WHERE price > 9.99").is_empty());
}

#[test]
fn eq_float_flagged() {
    let d = check("SELECT id FROM t WHERE price = 9.99");
    assert_eq!(d.len(), 1);
}

#[test]
fn neq_float_flagged() {
    let d = check("SELECT id FROM t WHERE ratio != 0.5");
    assert_eq!(d.len(), 1);
}

#[test]
fn diamond_neq_float_flagged() {
    let d = check("SELECT id FROM t WHERE rate <> 1.5");
    assert_eq!(d.len(), 1);
}

#[test]
fn float_in_string_not_flagged() {
    assert!(check("SELECT id FROM t WHERE name = '9.99'").is_empty());
}

#[test]
fn float_in_comment_not_flagged() {
    assert!(check("SELECT id FROM t -- where price = 9.99\nWHERE id > 1").is_empty());
}

#[test]
fn two_float_comparisons_flagged() {
    let d = check("SELECT id FROM t WHERE price = 9.99 AND ratio != 0.5");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_float_or_precision() {
    let d = check("SELECT id FROM t WHERE price = 9.99");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("float") || msg.contains("precision") || msg.contains("exact") || msg.contains("decimal"),
        "expected message to mention float/precision/exact, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM t WHERE price = 9.99");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Ambiguous/FloatingPointComparison");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM t WHERE price = 9.99");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn zero_point_zero_flagged() {
    let d = check("SELECT id FROM t WHERE ratio = 0.0");
    assert_eq!(d.len(), 1);
}

#[test]
fn negative_float_flagged() {
    let d = check("SELECT id FROM t WHERE ratio = -0.5");
    assert_eq!(d.len(), 1);
}
