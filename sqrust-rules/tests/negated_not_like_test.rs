use sqrust_core::FileContext;
use sqrust_rules::convention::negated_not_like::NegatedNotLike;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NegatedNotLike.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NegatedNotLike.name(), "Convention/NegatedNotLike");
}

#[test]
fn not_col_like_is_flagged() {
    let diags = check("SELECT * FROM t WHERE NOT col LIKE '%foo%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn col_not_like_is_not_flagged() {
    let diags = check("SELECT * FROM t WHERE col NOT LIKE '%foo%'");
    assert!(diags.is_empty());
}

#[test]
fn not_col_between_is_flagged() {
    let diags = check("SELECT * FROM t WHERE NOT col BETWEEN 1 AND 10");
    assert_eq!(diags.len(), 1);
}

#[test]
fn col_not_between_is_not_flagged() {
    let diags = check("SELECT * FROM t WHERE col NOT BETWEEN 1 AND 10");
    assert!(diags.is_empty());
}

#[test]
fn not_col_in_list_is_flagged() {
    let diags = check("SELECT * FROM t WHERE NOT col IN (1, 2, 3)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn col_not_in_list_is_not_flagged() {
    let diags = check("SELECT * FROM t WHERE col NOT IN (1, 2, 3)");
    assert!(diags.is_empty());
}

#[test]
fn not_col_equal_is_not_flagged() {
    // NOT on equality — no special negated form, should not flag
    let diags = check("SELECT * FROM t WHERE NOT col = 5");
    assert!(diags.is_empty());
}

#[test]
fn multiple_violations_in_one_query() {
    let sql = "SELECT * FROM t WHERE NOT a LIKE 'x' AND NOT b IN (1,2)";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn not_with_nested_paren_like() {
    // NOT (col LIKE '%foo%') — sqlparser may parse differently
    // This is an acceptable form that we do NOT flag (parens wrap the whole predicate)
    let diags = check("SELECT * FROM t WHERE NOT (col LIKE '%foo%')");
    // Either 0 or 1 is acceptable depending on parser representation;
    // the rule checks UnaryOp{Not, Like} directly — with parens sqlparser
    // produces UnaryOp{Not, Nested{Like}} so the inner Like is not directly matched.
    // We document that parens change the match: 0 violations expected here.
    assert!(diags.is_empty());
}

#[test]
fn in_case_when_condition() {
    let sql = "SELECT CASE WHEN NOT col LIKE 'x' THEN 1 ELSE 0 END FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn col_is_not_null_is_not_flagged() {
    // IS NOT NULL is not a NOT+predicate form
    let diags = check("SELECT * FROM t WHERE col IS NOT NULL");
    assert!(diags.is_empty());
}

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT !! FROM WHERE NOT");
    assert!(diags.is_empty());
}

#[test]
fn not_col_in_subquery_is_flagged() {
    let diags = check("SELECT * FROM t WHERE NOT col IN (SELECT id FROM s)");
    assert_eq!(diags.len(), 1);
}
