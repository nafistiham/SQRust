use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::except_all::ExceptAll;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ExceptAll.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ExceptAll.name(), "Structure/ExceptAll");
}

#[test]
fn except_all_one_violation() {
    let diags = check("SELECT a FROM t EXCEPT ALL SELECT a FROM s");
    assert_eq!(diags.len(), 1);
}

#[test]
fn except_without_all_no_violation() {
    let diags = check("SELECT a FROM t EXCEPT SELECT a FROM s");
    assert!(diags.is_empty());
}

#[test]
fn intersect_all_one_violation() {
    let diags = check("SELECT a FROM t INTERSECT ALL SELECT a FROM s");
    assert_eq!(diags.len(), 1);
}

#[test]
fn intersect_without_all_no_violation() {
    let diags = check("SELECT a FROM t INTERSECT SELECT a FROM s");
    assert!(diags.is_empty());
}

#[test]
fn union_all_no_violation() {
    let diags = check("SELECT a FROM t UNION ALL SELECT a FROM s");
    assert!(diags.is_empty());
}

#[test]
fn union_without_all_no_violation() {
    let diags = check("SELECT a FROM t UNION SELECT a FROM s");
    assert!(diags.is_empty());
}

#[test]
fn except_all_and_intersect_all_two_violations() {
    let diags = check(
        "SELECT a FROM t EXCEPT ALL SELECT a FROM s INTERSECT ALL SELECT a FROM u",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn except_all_and_intersect_without_all_one_violation() {
    let diags = check(
        "SELECT a FROM t EXCEPT ALL SELECT a FROM s INTERSECT SELECT a FROM u",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn except_all_in_subquery_one_violation() {
    let diags = check(
        "SELECT x FROM (SELECT a FROM t EXCEPT ALL SELECT a FROM s) sub",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn except_all_in_cte_one_violation() {
    let diags = check(
        "WITH c AS (SELECT a FROM t EXCEPT ALL SELECT a FROM s) SELECT * FROM c",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_no_violation() {
    let diags = check("NOT VALID SQL @@@");
    assert!(diags.is_empty());
}

#[test]
fn two_except_all_two_violations() {
    let diags = check(
        "SELECT a FROM t EXCEPT ALL SELECT a FROM s EXCEPT ALL SELECT a FROM u",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_mentions_all_or_portability() {
    let diags = check("SELECT a FROM t EXCEPT ALL SELECT a FROM s");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(msg.contains("ALL") || msg.contains("portab") || msg.contains("database"));
}
