use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::implicit_boolean_comparison::ImplicitBooleanComparison;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    ImplicitBooleanComparison.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        ImplicitBooleanComparison.name(),
        "Ambiguous/ImplicitBooleanComparison"
    );
}

#[test]
fn eq_true_uppercase_flagged() {
    let d = check("SELECT * FROM t WHERE active = TRUE");
    assert_eq!(d.len(), 1);
}

#[test]
fn eq_false_uppercase_flagged() {
    let d = check("SELECT * FROM t WHERE active = FALSE");
    assert_eq!(d.len(), 1);
}

#[test]
fn eq_true_lowercase_flagged() {
    let d = check("SELECT * FROM t WHERE active = true");
    assert_eq!(d.len(), 1);
}

#[test]
fn eq_false_lowercase_flagged() {
    let d = check("SELECT * FROM t WHERE active = false");
    assert_eq!(d.len(), 1);
}

#[test]
fn eq_true_mixedcase_flagged() {
    let d = check("SELECT * FROM t WHERE active = True");
    assert_eq!(d.len(), 1);
}

#[test]
fn neq_true_flagged() {
    let d = check("SELECT * FROM t WHERE active != TRUE");
    assert_eq!(d.len(), 1);
}

#[test]
fn neq_false_flagged() {
    let d = check("SELECT * FROM t WHERE active != FALSE");
    assert_eq!(d.len(), 1);
}

#[test]
fn diamond_neq_true_flagged() {
    let d = check("SELECT * FROM t WHERE active <> TRUE");
    assert_eq!(d.len(), 1);
}

#[test]
fn diamond_neq_false_flagged() {
    let d = check("SELECT * FROM t WHERE active <> FALSE");
    assert_eq!(d.len(), 1);
}

#[test]
fn no_spaces_eq_true_flagged() {
    let d = check("SELECT * FROM t WHERE active=TRUE");
    assert_eq!(d.len(), 1);
}

#[test]
fn is_true_not_flagged() {
    // IS TRUE is the recommended form — should not be flagged
    assert!(check("SELECT * FROM t WHERE active IS TRUE").is_empty());
}

#[test]
fn boolean_in_string_not_flagged() {
    assert!(check("SELECT * FROM t WHERE name = '= TRUE'").is_empty());
}

#[test]
fn boolean_in_comment_not_flagged() {
    assert!(check("SELECT * FROM t -- WHERE active = TRUE\nWHERE id > 0").is_empty());
}

#[test]
fn two_violations_flagged() {
    let d = check("SELECT * FROM t WHERE a = TRUE AND b = FALSE");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_redundant_or_dialect() {
    let d = check("SELECT * FROM t WHERE active = TRUE");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("redundant") || msg.contains("dialect") || msg.contains("true") || msg.contains("false"),
        "expected message to mention redundant/dialect/true/false, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT * FROM t WHERE active = TRUE");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Ambiguous/ImplicitBooleanComparison");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT * FROM t WHERE active = TRUE");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}
