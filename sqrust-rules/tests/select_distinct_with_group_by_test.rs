use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::select_distinct_with_group_by::SelectDistinctWithGroupBy;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SelectDistinctWithGroupBy.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        SelectDistinctWithGroupBy.name(),
        "Ambiguous/SelectDistinctWithGroupBy"
    );
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = SelectDistinctWithGroupBy.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn distinct_with_group_by_one_violation() {
    let diags = check("SELECT DISTINCT name FROM t GROUP BY name");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/SelectDistinctWithGroupBy");
}

#[test]
fn distinct_without_group_by_no_violation() {
    let diags = check("SELECT DISTINCT name FROM t");
    assert!(diags.is_empty());
}

#[test]
fn group_by_without_distinct_no_violation() {
    let diags = check("SELECT name FROM t GROUP BY name");
    assert!(diags.is_empty());
}

#[test]
fn no_select_no_violation() {
    let diags = check("UPDATE t SET a = 1");
    assert!(diags.is_empty());
}

#[test]
fn distinct_with_multiple_group_by_columns_violation() {
    let diags = check("SELECT DISTINCT a, b FROM t GROUP BY a, b");
    assert_eq!(diags.len(), 1);
}

#[test]
fn subquery_with_distinct_and_group_by_violation() {
    let diags = check(
        "SELECT x FROM (SELECT DISTINCT name FROM t GROUP BY name) sub",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn cte_with_distinct_and_group_by_violation() {
    let sql = "WITH cte AS (SELECT DISTINCT name FROM t GROUP BY name) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_all_with_group_by_no_violation() {
    // SELECT ALL is not SELECT DISTINCT — should not flag.
    let diags = check("SELECT ALL name FROM t GROUP BY name");
    assert!(diags.is_empty());
}

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT DISTINCT name FROM t GROUP BY name");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("DISTINCT") || diags[0].message.contains("GROUP BY"),
        "message was: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT DISTINCT name FROM t GROUP BY name");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn two_selects_both_distinct_group_by_two_violations() {
    // Two separate queries in one statement via UNION — each has DISTINCT + GROUP BY.
    let sql = "SELECT DISTINCT a FROM t GROUP BY a \
               UNION ALL \
               SELECT DISTINCT b FROM t GROUP BY b";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}
