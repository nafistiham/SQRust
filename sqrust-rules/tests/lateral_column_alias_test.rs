use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::lateral_column_alias::LateralColumnAlias;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    LateralColumnAlias.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(LateralColumnAlias.name(), "Structure/LateralColumnAlias");
}

#[test]
fn alias_used_in_where_one_violation() {
    let diags = check("SELECT a * 2 AS doubled FROM t WHERE doubled > 10");
    assert_eq!(diags.len(), 1);
}

#[test]
fn non_alias_column_in_where_no_violation() {
    let diags = check("SELECT a * 2 AS doubled FROM t WHERE a > 10");
    assert!(diags.is_empty());
}

#[test]
fn alias_used_in_group_by_one_violation() {
    let diags = check("SELECT a * 2 AS doubled FROM t GROUP BY doubled");
    assert_eq!(diags.len(), 1);
}

#[test]
fn alias_in_group_by_and_having_two_violations() {
    let diags = check(
        "SELECT a * 2 AS doubled, SUM(b) AS total FROM t GROUP BY doubled HAVING total > 100",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn plain_column_in_where_no_violation() {
    let diags = check("SELECT a FROM t WHERE a > 5");
    assert!(diags.is_empty());
}

#[test]
fn alias_in_order_by_no_violation() {
    let diags = check("SELECT a AS renamed FROM t ORDER BY renamed");
    assert!(diags.is_empty());
}

#[test]
fn alias_x_in_where_one_violation() {
    let diags = check("SELECT a AS x, b AS y FROM t WHERE x = 1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn alias_x_in_where_with_non_alias_column_one_violation() {
    let diags = check("SELECT a AS x FROM t WHERE x = 1 AND a > 0");
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_no_violation() {
    let diags = check("NOT VALID SQL @@@");
    assert!(diags.is_empty());
}

#[test]
fn alias_in_subquery_where_one_violation() {
    let diags = check(
        "SELECT outer_alias FROM (SELECT a * 2 AS doubled, b FROM t WHERE doubled > 5) sub",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn alias_in_cte_where_one_violation() {
    let diags = check(
        "WITH c AS (SELECT a * 2 AS doubled FROM t WHERE doubled > 5) SELECT * FROM c",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_where_group_by_having_no_violation() {
    let diags = check("SELECT a AS x FROM t");
    assert!(diags.is_empty());
}

#[test]
fn message_contains_alias_name() {
    let diags = check("SELECT a * 2 AS doubled FROM t WHERE doubled > 10");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(msg.contains("doubled"));
}
