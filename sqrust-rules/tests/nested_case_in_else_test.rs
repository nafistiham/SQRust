use sqrust_core::{FileContext, Rule};
use sqrust_rules::structure::nested_case_in_else::NestedCaseInElse;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    NestedCaseInElse.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NestedCaseInElse.name(), "Structure/NestedCaseInElse");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn simple_case_no_violation() {
    assert!(check("SELECT CASE WHEN x = 1 THEN 'a' ELSE 'b' END FROM t").is_empty());
}

#[test]
fn case_with_no_else_no_violation() {
    assert!(check("SELECT CASE WHEN x = 1 THEN 'a' END FROM t").is_empty());
}

#[test]
fn case_in_then_not_flagged() {
    assert!(check("SELECT CASE WHEN x = 1 THEN CASE WHEN y = 2 THEN 'a' ELSE 'b' END ELSE 'c' END FROM t").is_empty());
}

#[test]
fn case_nested_in_else_flagged() {
    let d = check("SELECT CASE WHEN x = 1 THEN 'a' ELSE CASE WHEN y = 2 THEN 'b' ELSE 'c' END END FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn double_nested_else_case_flagged_at_least_once() {
    let d = check("SELECT CASE WHEN a = 1 THEN 'x' ELSE CASE WHEN b = 2 THEN 'y' ELSE CASE WHEN c = 3 THEN 'z' ELSE 'w' END END END FROM t");
    assert!(d.len() >= 1);
}

#[test]
fn two_separate_nested_cases_flagged_twice() {
    let sql = "SELECT CASE WHEN a = 1 THEN 'x' ELSE CASE WHEN b = 2 THEN 'y' ELSE 'z' END END, CASE WHEN c = 3 THEN 'p' ELSE CASE WHEN d = 4 THEN 'q' ELSE 'r' END END FROM t";
    let d = check(sql);
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_else_or_case() {
    let d = check("SELECT CASE WHEN x = 1 THEN 'a' ELSE CASE WHEN y = 2 THEN 'b' ELSE 'c' END END FROM t");
    let msg = d[0].message.to_uppercase();
    assert!(msg.contains("ELSE") || msg.contains("CASE"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT CASE WHEN x = 1 THEN 'a' ELSE CASE WHEN y = 2 THEN 'b' ELSE 'c' END END FROM t");
    assert_eq!(d[0].rule, "Structure/NestedCaseInElse");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT CASE WHEN x = 1 THEN 'a' ELSE CASE WHEN y = 2 THEN 'b' ELSE 'c' END END FROM t");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn nested_case_in_subquery_flagged() {
    let d = check("SELECT * FROM (SELECT CASE WHEN x = 1 THEN 'a' ELSE CASE WHEN y = 2 THEN 'b' ELSE 'c' END END AS v FROM t) sub");
    assert_eq!(d.len(), 1);
}

#[test]
fn nested_case_in_where_flagged() {
    let d = check("SELECT id FROM t WHERE CASE WHEN x = 1 THEN 1 ELSE CASE WHEN y = 2 THEN 2 ELSE 0 END END > 0");
    assert_eq!(d.len(), 1);
}
