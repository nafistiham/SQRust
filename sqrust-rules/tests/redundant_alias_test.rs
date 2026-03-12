use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::redundant_alias::RedundantAlias;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    RedundantAlias.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(RedundantAlias.name(), "Convention/RedundantAlias");
}

#[test]
fn identical_alias_and_column_one_violation() {
    let sql = "SELECT col AS col FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("col"));
}

#[test]
fn different_alias_no_violation() {
    let sql = "SELECT col AS other FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_redundant_aliases_two_violations() {
    let sql = "SELECT a AS a, b AS b FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn one_redundant_one_not_one_violation() {
    let sql = "SELECT a AS a, b AS c FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("a"));
}

#[test]
fn case_insensitive_match_one_violation() {
    let sql = "SELECT Col AS col FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.to_lowercase().contains("col"));
}

#[test]
fn compound_identifier_last_part_matches_alias_one_violation() {
    let sql = "SELECT t.col AS col FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("col"));
}

#[test]
fn compound_identifier_alias_differs_no_violation() {
    let sql = "SELECT t.col AS t_col FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn no_alias_no_violation() {
    let sql = "SELECT col FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn redundant_alias_in_subquery_one_violation() {
    let sql = "SELECT x FROM (SELECT a AS a FROM t) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("a"));
}

#[test]
fn redundant_alias_in_cte_one_violation() {
    let sql = "WITH c AS (SELECT x AS x FROM t) SELECT x FROM c";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("x"));
}

#[test]
fn parse_error_returns_empty() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = RedundantAlias.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn multiple_columns_none_redundant_no_violation() {
    let sql = "SELECT a AS x, b AS y, c AS z FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_format_is_correct() {
    let sql = "SELECT foo AS foo FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "Column alias 'foo' is identical to the column name — alias is redundant"
    );
}
