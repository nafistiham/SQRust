use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::subquery_in_order_by::SubqueryInOrderBy;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SubqueryInOrderBy.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SubqueryInOrderBy.name(), "Ambiguous/SubqueryInOrderBy");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = SubqueryInOrderBy.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn no_order_by_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

#[test]
fn simple_order_by_no_violation() {
    let diags = check("SELECT id FROM t ORDER BY id");
    assert!(diags.is_empty());
}

#[test]
fn subquery_in_order_by_one_violation() {
    let diags = check("SELECT id FROM t ORDER BY (SELECT MAX(score) FROM scores)");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/SubqueryInOrderBy");
}

#[test]
fn subquery_in_where_not_flagged() {
    let diags = check(
        "SELECT id FROM t WHERE id IN (SELECT id FROM u) ORDER BY id",
    );
    assert!(diags.is_empty());
}

#[test]
fn subquery_in_select_not_flagged() {
    let diags = check("SELECT (SELECT 1) FROM t ORDER BY id");
    assert!(diags.is_empty());
}

#[test]
fn order_by_column_reference_no_violation() {
    let diags = check("SELECT id FROM t ORDER BY name DESC");
    assert!(diags.is_empty());
}

#[test]
fn two_subqueries_in_order_by_two_violations() {
    let diags = check(
        "SELECT id FROM t ORDER BY (SELECT MAX(a) FROM s1), (SELECT MIN(b) FROM s2)",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_mentions_subquery() {
    let diags = check("SELECT id FROM t ORDER BY (SELECT MAX(score) FROM scores)");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("subquery") || msg.contains("order by"),
        "expected message to mention 'subquery' or 'order by', got: {}",
        diags[0].message
    );
}

#[test]
fn line_nonzero() {
    let diags = check("SELECT id FROM t ORDER BY (SELECT MAX(score) FROM scores)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line must be >= 1");
}

#[test]
fn col_nonzero() {
    let diags = check("SELECT id FROM t ORDER BY (SELECT MAX(score) FROM scores)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1, "col must be >= 1");
}

#[test]
fn exists_in_order_by_flagged() {
    // EXISTS(...) used in ORDER BY is a subquery form — should be flagged
    let sql = "SELECT id FROM t ORDER BY EXISTS(SELECT 1 FROM t WHERE id = 1)";
    let ctx = FileContext::from_source(sql, "test.sql");
    // Parser may or may not accept this — only assert if it parsed.
    if ctx.parse_errors.is_empty() {
        let diags = SubqueryInOrderBy.check(&ctx);
        assert_eq!(diags.len(), 1, "EXISTS in ORDER BY should be flagged");
    }
}

#[test]
fn subquery_in_cte_order_by_flagged() {
    let sql = "
        WITH ranked AS (
            SELECT id, score FROM t ORDER BY (SELECT MAX(score) FROM scores)
        )
        SELECT id FROM ranked
    ";
    let diags = check(sql);
    assert_eq!(diags.len(), 1, "subquery in CTE ORDER BY should be flagged");
}
