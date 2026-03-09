use sqrust_core::{FileContext, Rule};
use sqrust_rules::convention::left_join::LeftJoin;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    LeftJoin.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(LeftJoin.name(), "Convention/LeftJoin");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn left_join_no_violation() {
    assert!(check("SELECT a.id FROM a LEFT JOIN b ON a.id = b.id").is_empty());
}

#[test]
fn inner_join_no_violation() {
    assert!(check("SELECT a.id FROM a INNER JOIN b ON a.id = b.id").is_empty());
}

#[test]
fn cross_join_no_violation() {
    assert!(check("SELECT a.id FROM a CROSS JOIN b").is_empty());
}

#[test]
fn right_join_flagged() {
    let d = check("SELECT a.id FROM a RIGHT JOIN b ON a.id = b.id");
    assert_eq!(d.len(), 1);
}

#[test]
fn right_outer_join_flagged() {
    let d = check("SELECT a.id FROM a RIGHT OUTER JOIN b ON a.id = b.id");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_right_joins_flagged() {
    let d = check("SELECT * FROM a RIGHT JOIN b ON a.id = b.id RIGHT JOIN c ON a.id = c.id");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_left() {
    let d = check("SELECT * FROM a RIGHT JOIN b ON a.id = b.id");
    assert!(d[0].message.to_uppercase().contains("LEFT"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT * FROM a RIGHT JOIN b ON a.id = b.id");
    assert_eq!(d[0].rule, "Convention/LeftJoin");
}

#[test]
fn points_to_right_keyword() {
    let d = check("SELECT * FROM a RIGHT JOIN b ON a.id = b.id");
    assert!(d[0].col >= 1);
    assert!(d[0].line >= 1);
}

#[test]
fn right_join_in_subquery_flagged() {
    let d = check("SELECT * FROM (SELECT * FROM a RIGHT JOIN b ON a.id = b.id) sub");
    assert_eq!(d.len(), 1);
}

#[test]
fn right_in_string_not_flagged() {
    assert!(check("SELECT 'RIGHT JOIN' FROM t").is_empty());
}

#[test]
fn right_in_comment_not_flagged() {
    assert!(check("SELECT a FROM t -- RIGHT JOIN example").is_empty());
}
