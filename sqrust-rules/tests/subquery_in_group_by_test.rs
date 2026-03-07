use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::subquery_in_group_by::SubqueryInGroupBy;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SubqueryInGroupBy.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SubqueryInGroupBy.name(), "Ambiguous/SubqueryInGroupBy");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = SubqueryInGroupBy.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn subquery_in_group_by_one_violation() {
    let diags = check("SELECT col, COUNT(*) FROM t GROUP BY (SELECT 1)");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/SubqueryInGroupBy");
}

#[test]
fn column_in_group_by_no_violation() {
    let diags = check("SELECT col, COUNT(*) FROM t GROUP BY col");
    assert!(diags.is_empty());
}

#[test]
fn expression_in_group_by_no_violation() {
    let diags = check("SELECT col + 1, COUNT(*) FROM t GROUP BY col + 1");
    assert!(diags.is_empty());
}

#[test]
fn no_group_by_no_violation() {
    let diags = check("SELECT col FROM t WHERE col = 1");
    assert!(diags.is_empty());
}

#[test]
fn in_subquery_in_group_by_violation() {
    // col IN (SELECT id FROM t) used as a GROUP BY expression
    let diags = check("SELECT col IN (SELECT id FROM t2), COUNT(*) FROM t GROUP BY col IN (SELECT id FROM t2)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_subqueries_in_group_by_two_violations() {
    let diags = check("SELECT a, b, COUNT(*) FROM t GROUP BY (SELECT 1), (SELECT 2)");
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_mentions_group_by_or_non_standard() {
    let diags = check("SELECT col, COUNT(*) FROM t GROUP BY (SELECT 1)");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message.to_lowercase();
    assert!(
        msg.contains("group by") || msg.contains("non-standard") || msg.contains("subquery"),
        "expected message to mention 'group by', 'non-standard', or 'subquery', got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT col, COUNT(*) FROM t GROUP BY (SELECT 1)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line must be >= 1");
    assert!(diags[0].col >= 1, "col must be >= 1");
}

#[test]
fn function_in_group_by_no_violation() {
    // A function call is not a subquery
    let diags = check("SELECT UPPER(col), COUNT(*) FROM t GROUP BY UPPER(col)");
    assert!(diags.is_empty());
}

#[test]
fn subquery_in_where_not_flagged() {
    // Subquery in WHERE should not trigger this rule
    let diags = check("SELECT col FROM t WHERE col IN (SELECT id FROM t2) GROUP BY col");
    assert!(diags.is_empty());
}

#[test]
fn group_by_all_no_violation() {
    // GROUP BY ALL is a valid sqlparser variant — should not be flagged
    // Note: sqlparser may or may not parse this, so we only assert no false positive
    let sql = "SELECT col, COUNT(*) FROM t GROUP BY ALL";
    let ctx = FileContext::from_source(sql, "test.sql");
    // If it parsed successfully, no violation should be emitted
    if ctx.parse_errors.is_empty() {
        let diags = SubqueryInGroupBy.check(&ctx);
        assert!(diags.is_empty(), "GROUP BY ALL should not be flagged");
    }
}

#[test]
fn multiple_columns_one_subquery_one_violation() {
    let diags = check("SELECT a, b, COUNT(*) FROM t GROUP BY a, (SELECT 1), b");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nested_subquery_in_expression_flagged() {
    // (SELECT 1) + 1 as GROUP BY expr — contains a subquery inside a BinaryOp
    let diags = check("SELECT a FROM t GROUP BY (SELECT 1) + 1");
    assert_eq!(diags.len(), 1);
}
