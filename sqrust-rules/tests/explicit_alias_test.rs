use sqrust_core::{FileContext, Rule};
use sqrust_rules::convention::explicit_alias::ExplicitAlias;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    ExplicitAlias.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ExplicitAlias.name(), "Convention/ExplicitAlias");
}

#[test]
fn explicit_alias_no_violation() {
    assert!(check("SELECT id FROM t AS alias").is_empty());
}

#[test]
fn no_alias_no_violation() {
    assert!(check("SELECT id FROM t WHERE id = 1").is_empty());
}

#[test]
fn join_with_as_no_violation() {
    assert!(check("SELECT t.id FROM t AS t1 JOIN u AS u1 ON t1.id = u1.t_id").is_empty());
}

#[test]
fn implicit_table_alias_flagged() {
    let d = check("SELECT id FROM orders o WHERE o.id = 1");
    assert_eq!(d.len(), 1);
}

#[test]
fn implicit_join_alias_flagged() {
    let d = check("SELECT t.id FROM t JOIN u u1 ON t.id = u1.t_id");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_implicit_aliases_flagged() {
    let d = check("SELECT a.id FROM accounts a JOIN orders o ON a.id = o.account_id");
    assert_eq!(d.len(), 2);
}

#[test]
fn subquery_with_implicit_alias_flagged() {
    let d = check("SELECT id FROM (SELECT id FROM t) sub");
    assert_eq!(d.len(), 1);
}

#[test]
fn subquery_with_explicit_alias_no_violation() {
    assert!(check("SELECT id FROM (SELECT id FROM t) AS sub").is_empty());
}

#[test]
fn message_mentions_as() {
    let d = check("SELECT id FROM t alias");
    assert_eq!(d.len(), 1);
    assert!(
        d[0].message.to_uppercase().contains("AS"),
        "expected message to mention AS, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM t alias");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Convention/ExplicitAlias");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM t alias");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn alias_in_string_not_flagged() {
    assert!(check("SELECT 'FROM t alias' FROM t AS real_alias").is_empty());
}

#[test]
fn lateral_join_with_explicit_alias_no_violation() {
    assert!(check("SELECT t.id FROM t JOIN u AS u1 ON t.id = u1.id").is_empty());
}
