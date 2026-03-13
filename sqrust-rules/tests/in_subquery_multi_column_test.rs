use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::in_subquery_multi_column::InSubqueryMultiColumn;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    InSubqueryMultiColumn.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(InSubqueryMultiColumn.name(), "Ambiguous/InSubqueryMultiColumn");
}

#[test]
fn in_subquery_two_columns_one_violation() {
    let diags = check("SELECT * FROM t WHERE id IN (SELECT a, b FROM s)");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/InSubqueryMultiColumn");
}

#[test]
fn in_subquery_one_column_no_violation() {
    let diags = check("SELECT * FROM t WHERE id IN (SELECT a FROM s)");
    assert!(diags.is_empty());
}

#[test]
fn in_subquery_wildcard_no_violation() {
    let diags = check("SELECT * FROM t WHERE id IN (SELECT * FROM s)");
    assert!(diags.is_empty());
}

#[test]
fn in_subquery_three_columns_one_violation() {
    let diags = check("SELECT * FROM t WHERE id IN (SELECT a, b, c FROM s)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_in_subquery_two_columns_one_violation() {
    let diags = check("SELECT * FROM t WHERE id NOT IN (SELECT a, b FROM s)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn in_value_list_no_violation() {
    let diags = check("SELECT * FROM t WHERE id IN (1, 2, 3)");
    assert!(diags.is_empty());
}

#[test]
fn in_subquery_two_columns_in_cte_one_violation() {
    let diags = check(
        "WITH c AS (SELECT * FROM t WHERE id IN (SELECT a, b FROM s)) SELECT * FROM c",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn in_subquery_two_columns_in_from_subquery_one_violation() {
    let diags = check(
        "SELECT x FROM (SELECT * FROM t WHERE id IN (SELECT a, b FROM s)) sub",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn correlated_subquery_one_column_no_violation() {
    let diags = check(
        "SELECT * FROM t WHERE id IN (SELECT a FROM s WHERE s.b = t.b)",
    );
    assert!(diags.is_empty());
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = InSubqueryMultiColumn.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn row_constructor_in_subquery_two_columns_no_violation() {
    // (a, b) IN (SELECT a, b FROM s) is a row-constructor pattern, legitimate in some databases.
    // sqlparser may not parse this as Expr::InSubquery at all — expect 0 violations.
    let ctx = FileContext::from_source(
        "SELECT * FROM t WHERE (a, b) IN (SELECT a, b FROM s)",
        "test.sql",
    );
    // Either it doesn't parse or produces a different AST node — no violation expected.
    let diags = InSubqueryMultiColumn.check(&ctx);
    assert!(diags.is_empty());
}

#[test]
fn union_subquery_two_columns_one_violation() {
    let diags = check(
        "SELECT * FROM t WHERE id IN (SELECT a, b FROM s1 UNION SELECT a, b FROM s2)",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_expected_text() {
    let diags = check("SELECT * FROM t WHERE id IN (SELECT a, b FROM s)");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("column") || msg.contains("columns"),
        "message was: {msg}"
    );
}
