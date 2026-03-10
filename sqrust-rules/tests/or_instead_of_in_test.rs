use sqrust_core::{FileContext, Rule};
use sqrust_rules::convention::or_instead_of_in::OrInsteadOfIn;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    OrInsteadOfIn.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(OrInsteadOfIn.name(), "Convention/OrInsteadOfIn");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn two_or_conditions_same_col_no_violation() {
    assert!(check("SELECT id FROM t WHERE status = 'a' OR status = 'b'").is_empty());
}

#[test]
fn three_or_conditions_same_col_flagged() {
    let d = check("SELECT id FROM t WHERE status = 'a' OR status = 'b' OR status = 'c'");
    assert_eq!(d.len(), 1);
}

#[test]
fn four_or_conditions_same_col_flagged_once() {
    let d = check("SELECT id FROM t WHERE s = 1 OR s = 2 OR s = 3 OR s = 4");
    assert_eq!(d.len(), 1);
}

#[test]
fn three_different_cols_no_violation() {
    assert!(check("SELECT id FROM t WHERE a = 1 OR b = 2 OR c = 3").is_empty());
}

#[test]
fn two_same_one_diff_no_violation() {
    assert!(check("SELECT id FROM t WHERE a = 1 OR b = 2 OR a = 3").is_empty());
}

#[test]
fn in_clause_used_correctly_no_violation() {
    assert!(check("SELECT id FROM t WHERE status IN ('a', 'b', 'c')").is_empty());
}

#[test]
fn three_or_in_having_flagged() {
    let d = check("SELECT dept FROM t GROUP BY dept HAVING dept = 'a' OR dept = 'b' OR dept = 'c'");
    assert_eq!(d.len(), 1);
}

#[test]
fn message_mentions_in() {
    let d = check("SELECT id FROM t WHERE x = 1 OR x = 2 OR x = 3");
    assert_eq!(d.len(), 1);
    assert!(
        d[0].message.to_uppercase().contains("IN"),
        "expected message to mention IN, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM t WHERE x = 1 OR x = 2 OR x = 3");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Convention/OrInsteadOfIn");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM t WHERE x = 1 OR x = 2 OR x = 3");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn qualified_col_three_or_flagged() {
    let d = check("SELECT id FROM t WHERE t.status = 'a' OR t.status = 'b' OR t.status = 'c'");
    assert_eq!(d.len(), 1);
}

#[test]
fn mixed_qualified_unqualified_three_no_violation() {
    // Different column name forms — conservative: don't flag if mixed qualified/unqualified
    // (treating t.status and status as different for simplicity)
    assert!(check("SELECT id FROM t WHERE t.status = 'a' OR status = 'b' OR t.status = 'c'").is_empty());
}
