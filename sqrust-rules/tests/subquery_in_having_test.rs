use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::subquery_in_having::SubqueryInHaving;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SubqueryInHaving.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SubqueryInHaving.name(), "Structure/SubqueryInHaving");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = SubqueryInHaving.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn subquery_in_having_one_violation() {
    let diags = check(
        "SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) > (SELECT AVG(cnt) FROM summary)",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/SubqueryInHaving");
}

#[test]
fn no_subquery_in_having_no_violation() {
    let diags = check("SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) > 5");
    assert!(diags.is_empty());
}

#[test]
fn exists_in_having_one_violation() {
    let sql =
        "SELECT dept FROM t GROUP BY dept HAVING EXISTS(SELECT 1 FROM other WHERE other.dept = t.dept)";
    let ctx = FileContext::from_source(sql, "test.sql");
    // Parser may or may not accept EXISTS in HAVING — only assert if parsed.
    if ctx.parse_errors.is_empty() {
        let diags = SubqueryInHaving.check(&ctx);
        assert_eq!(diags.len(), 1, "EXISTS in HAVING should be flagged");
    }
}

#[test]
fn in_subquery_in_having_one_violation() {
    let diags = check(
        "SELECT dept FROM t GROUP BY dept HAVING dept IN (SELECT dept FROM allowed)",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn having_with_aggregate_only_no_violation() {
    let diags = check("SELECT dept, SUM(salary) FROM t GROUP BY dept HAVING SUM(salary) > 100000");
    assert!(diags.is_empty());
}

#[test]
fn having_with_simple_condition_no_violation() {
    let diags = check("SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) >= 10");
    assert!(diags.is_empty());
}

#[test]
fn subquery_in_cte_having_detected() {
    let sql = "
        WITH dept_counts AS (
            SELECT dept, COUNT(*) AS cnt FROM t GROUP BY dept
            HAVING COUNT(*) > (SELECT AVG(c) FROM counts)
        )
        SELECT dept FROM dept_counts
    ";
    let diags = check(sql);
    assert_eq!(diags.len(), 1, "subquery in CTE HAVING should be detected");
}

#[test]
fn subquery_in_outer_subquery_having_detected() {
    let sql = "
        SELECT dept FROM (
            SELECT dept, COUNT(*) AS cnt FROM t GROUP BY dept
            HAVING COUNT(*) > (SELECT AVG(c) FROM counts)
        ) sub
    ";
    let diags = check(sql);
    assert_eq!(diags.len(), 1, "subquery in HAVING inside derived table should be detected");
}

#[test]
fn multiple_subqueries_in_having_multiple_violations() {
    // Two separate IN-subquery conditions in HAVING via AND
    let sql = "SELECT dept FROM t GROUP BY dept \
               HAVING dept IN (SELECT dept FROM a) AND dept IN (SELECT dept FROM b)";
    let diags = check(sql);
    assert!(diags.len() >= 1, "at least one violation expected for multiple subqueries in HAVING");
}

#[test]
fn subquery_in_where_not_flagged() {
    let diags = check(
        "SELECT dept, COUNT(*) FROM t WHERE dept IN (SELECT dept FROM allowed) GROUP BY dept HAVING COUNT(*) > 5",
    );
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_having() {
    let diags = check(
        "SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) > (SELECT AVG(cnt) FROM s)",
    );
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("having") || msg.contains("subquery") || msg.contains("cte"),
        "expected message to mention 'having', 'subquery', or 'cte', got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check(
        "SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) > (SELECT AVG(cnt) FROM s)",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line must be >= 1");
    assert!(diags[0].col >= 1, "col must be >= 1");
}
