use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::subquery_without_alias::SubqueryWithoutAlias;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SubqueryWithoutAlias.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SubqueryWithoutAlias.name(), "Lint/SubqueryWithoutAlias");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = SubqueryWithoutAlias.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn subquery_with_as_alias_no_violation() {
    let sql = "SELECT * FROM (SELECT 1 AS n) AS subq";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn subquery_without_alias_one_violation() {
    let sql = "SELECT * FROM (SELECT 1 AS n)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn join_subquery_with_alias_no_violation() {
    let sql = "SELECT * FROM t JOIN (SELECT id FROM u) AS j ON t.id = j.id";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn join_subquery_without_alias_one_violation() {
    let sql = "SELECT * FROM t JOIN (SELECT id FROM u) ON t.id = u.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn simple_table_reference_no_violation() {
    let sql = "SELECT * FROM my_table WHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn cte_reference_no_violation() {
    let sql = "WITH my_cte AS (SELECT id FROM t) SELECT * FROM my_cte";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiple_unaliased_subqueries_multiple_violations() {
    let sql = "SELECT * FROM (SELECT 1 AS a), (SELECT 2 AS b)";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn nested_subquery_without_alias_inner_flagged() {
    // The outer subquery has an alias, but the inner one does not.
    let sql = "SELECT * FROM (SELECT * FROM (SELECT 1 AS n)) AS outer_alias";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn line_col_is_non_zero() {
    let sql = "SELECT * FROM (SELECT 1 AS n)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn message_format_correct() {
    let sql = "SELECT * FROM (SELECT 1 AS n)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "Derived table (subquery in FROM) has no alias; add an alias for portability"
    );
}

#[test]
fn subquery_with_alias_without_as_keyword_no_violation() {
    // `(SELECT 1) subq` — alias without AS is still an alias
    let sql = "SELECT * FROM (SELECT 1) subq";
    let diags = check(sql);
    assert!(diags.is_empty());
}
