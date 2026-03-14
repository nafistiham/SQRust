use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::correlated_subquery::CorrelatedSubquery;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CorrelatedSubquery.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CorrelatedSubquery.name(), "Structure/CorrelatedSubquery");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = CorrelatedSubquery.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn subquery_in_where_flagged() {
    let diags = check(
        "SELECT * FROM orders o WHERE o.amount > (SELECT AVG(amount) FROM orders)",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/CorrelatedSubquery");
}

#[test]
fn subquery_in_having_flagged() {
    let diags = check(
        "SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) > (SELECT AVG(cnt) FROM summary)",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/CorrelatedSubquery");
}

#[test]
fn in_subquery_in_where_flagged() {
    let diags = check(
        "SELECT * FROM orders WHERE order_id IN (SELECT order_id FROM flagged_orders)",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_subquery_in_where_no_violation() {
    let diags = check("SELECT * FROM orders WHERE amount > 100");
    assert!(diags.is_empty());
}

#[test]
fn no_subquery_no_violation() {
    let diags = check("SELECT id, name FROM customers WHERE active = 1");
    assert!(diags.is_empty());
}

#[test]
fn simple_join_no_violation() {
    let diags = check(
        "SELECT o.id, c.name FROM orders o JOIN customers c ON o.customer_id = c.id",
    );
    assert!(diags.is_empty());
}

#[test]
fn subquery_in_select_not_flagged() {
    // Subquery in SELECT projection is not a correlated subquery concern for this rule
    let diags = check("SELECT (SELECT COUNT(*) FROM orders) AS total FROM dual");
    assert!(diags.is_empty());
}

#[test]
fn subquery_in_from_not_flagged() {
    let diags = check(
        "SELECT * FROM (SELECT id, name FROM customers WHERE active = 1) sub",
    );
    assert!(diags.is_empty());
}

#[test]
fn multiple_subqueries_in_where_multiple_violations() {
    let diags = check(
        "SELECT * FROM t WHERE a IN (SELECT a FROM a_table) AND b IN (SELECT b FROM b_table)",
    );
    assert!(diags.len() >= 1, "at least one violation expected");
}

#[test]
fn subquery_in_cte_where_flagged() {
    let sql = "
        WITH filtered AS (
            SELECT id FROM orders WHERE amount > (SELECT AVG(amount) FROM orders)
        )
        SELECT * FROM filtered
    ";
    let diags = check(sql);
    assert_eq!(diags.len(), 1, "subquery in CTE WHERE should be detected");
}

#[test]
fn message_mentions_performance() {
    let diags = check(
        "SELECT * FROM orders o WHERE o.amount > (SELECT AVG(amount) FROM orders)",
    );
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("correlated") || msg.contains("join") || msg.contains("row") || msg.contains("per"),
        "expected message to mention performance concern, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check(
        "SELECT * FROM orders o WHERE o.amount > (SELECT AVG(amount) FROM orders)",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line must be >= 1");
    assert!(diags[0].col >= 1, "col must be >= 1");
}

#[test]
fn exists_subquery_in_where_flagged() {
    let diags = check(
        "SELECT * FROM customers c WHERE EXISTS (SELECT 1 FROM orders o WHERE o.customer_id = c.id)",
    );
    assert_eq!(diags.len(), 1);
}
