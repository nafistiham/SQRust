use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::case_else::CaseElse;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CaseElse.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CaseElse.name(), "Convention/CaseElse");
}

#[test]
fn searched_case_without_else_one_violation() {
    let diags = check("SELECT CASE WHEN x > 1 THEN 'yes' END FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn searched_case_with_else_no_violation() {
    let diags = check("SELECT CASE WHEN x > 1 THEN 'yes' ELSE 'no' END FROM t");
    assert!(diags.is_empty());
}

#[test]
fn simple_case_without_else_one_violation() {
    let diags = check("SELECT CASE x WHEN 1 THEN 'one' END FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn simple_case_with_else_no_violation() {
    let diags = check("SELECT CASE x WHEN 1 THEN 'one' ELSE 'other' END FROM t");
    assert!(diags.is_empty());
}

#[test]
fn case_in_where_clause_one_violation() {
    let diags = check(
        "SELECT col FROM t WHERE CASE WHEN col > 0 THEN 1 END = 1",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_in_having_clause_one_violation() {
    let diags = check(
        "SELECT col, COUNT(*) FROM t GROUP BY col HAVING CASE WHEN COUNT(*) > 1 THEN 1 END = 1",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn nested_case_outer_has_else_inner_does_not_one_violation() {
    let diags = check(
        "SELECT CASE WHEN x > 1 THEN CASE WHEN y > 1 THEN 'a' END ELSE 'no' END FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn nested_case_both_have_else_no_violation() {
    let diags = check(
        "SELECT CASE WHEN x > 1 THEN CASE WHEN y > 1 THEN 'a' ELSE 'b' END ELSE 'no' END FROM t",
    );
    assert!(diags.is_empty());
}

#[test]
fn nested_case_both_missing_else_two_violations() {
    let diags = check(
        "SELECT CASE WHEN x > 1 THEN CASE WHEN y > 1 THEN 'a' END END FROM t",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = CaseElse.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn message_format_is_correct() {
    let diags = check("SELECT CASE WHEN x > 1 THEN 'yes' END FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "CASE expression has no ELSE clause; unmatched conditions will return NULL"
    );
}

#[test]
fn case_in_order_by_one_violation() {
    let diags = check(
        "SELECT col FROM t ORDER BY CASE WHEN col > 0 THEN 1 END",
    );
    assert_eq!(diags.len(), 1);
}
