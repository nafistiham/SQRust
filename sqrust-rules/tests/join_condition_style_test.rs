use sqrust_core::{FileContext, Rule};
use sqrust_rules::convention::join_condition_style::JoinConditionStyle;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    JoinConditionStyle.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(JoinConditionStyle.name(), "Convention/JoinConditionStyle");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn join_with_on_no_violation() {
    assert!(check("SELECT * FROM t1 JOIN t2 ON t1.id = t2.id").is_empty());
}

#[test]
fn no_join_where_no_violation() {
    assert!(check("SELECT * FROM t WHERE id > 1").is_empty());
}

#[test]
fn single_table_where_no_violation() {
    assert!(check("SELECT * FROM t WHERE t.id > 1 AND t.name = 'foo'").is_empty());
}

#[test]
fn cross_table_eq_in_where_flagged() {
    let d = check("SELECT * FROM t1, t2 WHERE t1.id = t2.id");
    assert_eq!(d.len(), 1);
}

#[test]
fn cross_table_eq_in_where_with_explicit_join_flagged() {
    let d = check("SELECT * FROM t1 JOIN t2 ON TRUE WHERE t1.id = t2.fk");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_cross_table_eqs_in_where_flagged() {
    let d = check("SELECT * FROM t1, t2, t3 WHERE t1.id = t2.id AND t2.id = t3.id");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_on() {
    let d = check("SELECT * FROM t1, t2 WHERE t1.id = t2.id");
    assert!(d[0].message.to_uppercase().contains("ON"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT * FROM t1, t2 WHERE t1.id = t2.id");
    assert_eq!(d[0].rule, "Convention/JoinConditionStyle");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT * FROM t1, t2 WHERE t1.id = t2.id");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn same_table_eq_in_where_no_violation() {
    assert!(check("SELECT * FROM t WHERE t.a = t.b").is_empty());
}

#[test]
fn cross_table_eq_in_subquery_where_flagged() {
    let d = check("SELECT * FROM (SELECT * FROM a, b WHERE a.id = b.id) sub");
    assert_eq!(d.len(), 1);
}

#[test]
fn full_join_with_on_and_same_table_where_no_violation() {
    // Filter in WHERE references only one table — not a join condition
    assert!(check("SELECT * FROM t1 FULL JOIN t2 ON t1.id = t2.id WHERE t1.active = 1").is_empty());
}
