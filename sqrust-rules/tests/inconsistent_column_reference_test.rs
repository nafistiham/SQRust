use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::inconsistent_column_reference::InconsistentColumnReference;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    InconsistentColumnReference.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(InconsistentColumnReference.name(), "Ambiguous/InconsistentColumnReference");
}

#[test]
fn all_named_order_by_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY name, age").is_empty());
}

#[test]
fn all_positional_order_by_no_violation() {
    assert!(check("SELECT id, name FROM t ORDER BY 1, 2").is_empty());
}

#[test]
fn mixed_in_order_by_flagged() {
    let d = check("SELECT id, name FROM t ORDER BY 1, name");
    assert_eq!(d.len(), 1);
}

#[test]
fn all_named_group_by_no_violation() {
    assert!(check("SELECT dept, COUNT(*) FROM t GROUP BY dept").is_empty());
}

#[test]
fn all_positional_group_by_no_violation() {
    assert!(check("SELECT dept, COUNT(*) FROM t GROUP BY 1").is_empty());
}

#[test]
fn mixed_in_group_by_flagged() {
    let d = check("SELECT dept, region, COUNT(*) FROM t GROUP BY 1, region");
    assert_eq!(d.len(), 1);
}

#[test]
fn mixed_in_both_clauses_flagged_twice() {
    let d = check("SELECT dept, region FROM t GROUP BY 1, region ORDER BY 1, dept");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_positional_or_reference() {
    let d = check("SELECT id, name FROM t ORDER BY 1, name");
    let msg = d[0].message.to_lowercase();
    assert!(msg.contains("positional") || msg.contains("reference") || msg.contains("consistent") || msg.contains("mix"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id, name FROM t ORDER BY 1, name");
    assert_eq!(d[0].rule, "Ambiguous/InconsistentColumnReference");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id, name FROM t ORDER BY 1, name");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn number_in_string_not_counted() {
    assert!(check("SELECT '1' FROM t ORDER BY name, age").is_empty());
}

#[test]
fn no_order_by_no_violation() {
    assert!(check("SELECT * FROM t WHERE id > 1").is_empty());
}

#[test]
fn no_group_by_no_violation() {
    assert!(check("SELECT * FROM t").is_empty());
}
