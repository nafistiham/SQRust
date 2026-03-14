use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::select_distinct_order_by::SelectDistinctOrderBy;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SelectDistinctOrderBy.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelectDistinctOrderBy.name(), "Ambiguous/SelectDistinctOrderBy");
}

#[test]
fn distinct_with_order_by_not_in_select_violation() {
    let diags = check("SELECT DISTINCT a FROM t ORDER BY b");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/SelectDistinctOrderBy");
}

#[test]
fn distinct_with_order_by_in_select_no_violation() {
    let diags = check("SELECT DISTINCT a, b FROM t ORDER BY b");
    assert!(diags.is_empty());
}

#[test]
fn no_distinct_order_by_not_in_select_no_violation() {
    // Only flag DISTINCT queries
    let diags = check("SELECT a FROM t ORDER BY b");
    assert!(diags.is_empty());
}

#[test]
fn distinct_no_order_by_no_violation() {
    let diags = check("SELECT DISTINCT a FROM t");
    assert!(diags.is_empty());
}

#[test]
fn distinct_with_alias_order_by_alias_no_violation() {
    let diags = check("SELECT DISTINCT a AS x FROM t ORDER BY x");
    assert!(diags.is_empty());
}

#[test]
fn distinct_wildcard_no_violation() {
    // Wildcard covers all columns, so ORDER BY any column is fine
    let diags = check("SELECT DISTINCT * FROM t ORDER BY a");
    assert!(diags.is_empty());
}

#[test]
fn distinct_multiple_order_by_one_missing_violation() {
    // ORDER BY a is fine (in select), but ORDER BY b is not
    let diags = check("SELECT DISTINCT a FROM t ORDER BY a, b");
    assert_eq!(diags.len(), 1);
}

#[test]
fn distinct_with_order_by_same_col_no_violation() {
    let diags = check("SELECT DISTINCT a FROM t ORDER BY a");
    assert!(diags.is_empty());
}

#[test]
fn parse_error_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = SelectDistinctOrderBy.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn message_mentions_column_name() {
    let diags = check("SELECT DISTINCT a FROM t ORDER BY b");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('b'),
        "Expected message to contain 'b', got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT DISTINCT a FROM t ORDER BY b");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn distinct_in_cte_with_bad_order_by_violation() {
    let sql = "WITH cte AS (SELECT DISTINCT a FROM t ORDER BY b) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
