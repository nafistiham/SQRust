use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::scalar_subquery_in_select::ScalarSubqueryInSelect;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ScalarSubqueryInSelect.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ScalarSubqueryInSelect.name(), "Structure/ScalarSubqueryInSelect");
}

#[test]
fn no_scalar_subqueries_no_violation() {
    let diags = check("SELECT id, name, status FROM customers WHERE active = 1");
    assert!(diags.is_empty());
}

#[test]
fn one_scalar_subquery_no_violation() {
    let diags = check(
        "SELECT id, (SELECT COUNT(*) FROM orders WHERE customer_id = c.id) AS order_count FROM customers c",
    );
    assert!(diags.is_empty());
}

#[test]
fn two_scalar_subqueries_no_violation() {
    let diags = check(
        "SELECT id, \
         (SELECT COUNT(*) FROM orders WHERE customer_id = c.id) AS order_count, \
         (SELECT MAX(amount) FROM orders WHERE customer_id = c.id) AS max_order \
         FROM customers c",
    );
    assert!(diags.is_empty());
}

#[test]
fn three_scalar_subqueries_flagged() {
    let diags = check(
        "SELECT id, \
         (SELECT COUNT(*) FROM orders WHERE customer_id = c.id) AS order_count, \
         (SELECT MAX(amount) FROM orders WHERE customer_id = c.id) AS max_order, \
         (SELECT MIN(amount) FROM orders WHERE customer_id = c.id) AS min_order \
         FROM customers c",
    );
    assert_eq!(diags.len(), 1, "expected 1 violation for 3 scalar subqueries");
}

#[test]
fn four_scalar_subqueries_flagged() {
    let diags = check(
        "SELECT id, \
         (SELECT COUNT(*) FROM orders WHERE customer_id = c.id) AS a, \
         (SELECT MAX(amount) FROM orders WHERE customer_id = c.id) AS b, \
         (SELECT MIN(amount) FROM orders WHERE customer_id = c.id) AS c2, \
         (SELECT SUM(amount) FROM orders WHERE customer_id = c.id) AS d \
         FROM customers c",
    );
    assert_eq!(diags.len(), 1, "expected 1 violation for 4 scalar subqueries");
}

#[test]
fn violation_rule_name_is_correct() {
    let diags = check(
        "SELECT id, \
         (SELECT COUNT(*) FROM o WHERE o.cid = c.id), \
         (SELECT MAX(v) FROM o WHERE o.cid = c.id), \
         (SELECT MIN(v) FROM o WHERE o.cid = c.id) \
         FROM c",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/ScalarSubqueryInSelect");
}

#[test]
fn message_contains_count() {
    let diags = check(
        "SELECT id, \
         (SELECT COUNT(*) FROM o WHERE o.cid = c.id), \
         (SELECT MAX(v) FROM o WHERE o.cid = c.id), \
         (SELECT MIN(v) FROM o WHERE o.cid = c.id) \
         FROM c",
    );
    assert_eq!(diags.len(), 1);
    // Message should mention the count "3"
    assert!(
        diags[0].message.contains('3'),
        "expected message to contain the count, got: {}",
        diags[0].message
    );
}

#[test]
fn message_mentions_performance() {
    let diags = check(
        "SELECT id, \
         (SELECT COUNT(*) FROM o WHERE o.cid = c.id), \
         (SELECT MAX(v) FROM o WHERE o.cid = c.id), \
         (SELECT MIN(v) FROM o WHERE o.cid = c.id) \
         FROM c",
    );
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("row") || msg.contains("performance") || msg.contains("per"),
        "expected message to mention performance, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check(
        "SELECT id, \
         (SELECT COUNT(*) FROM o WHERE o.cid = c.id), \
         (SELECT MAX(v) FROM o WHERE o.cid = c.id), \
         (SELECT MIN(v) FROM o WHERE o.cid = c.id) \
         FROM c",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn select_in_string_literal_not_counted() {
    // '(SELECT' inside a string should not be counted as a scalar subquery
    let diags = check(
        "SELECT id, \
         '(SELECT x FROM y)', \
         '(SELECT a FROM b)', \
         (SELECT COUNT(*) FROM o WHERE o.cid = c.id), \
         (SELECT MAX(v) FROM o WHERE o.cid = c.id) \
         FROM c",
    );
    // Only 2 real scalar subqueries, below threshold
    assert!(diags.is_empty(), "string literals should not be counted");
}

#[test]
fn select_in_line_comment_not_counted() {
    // '(SELECT' inside a comment should not be counted
    let sql = "SELECT id,\n-- (SELECT fake FROM t),\n-- (SELECT fake2 FROM t),\n(SELECT COUNT(*) FROM o),\n(SELECT MAX(v) FROM o),\n(SELECT MIN(v) FROM o)\nFROM c";
    let diags = check(sql);
    // Only 3 real scalar subqueries, should be flagged
    assert_eq!(diags.len(), 1, "comments should not be counted but real subqueries should");
}

#[test]
fn subquery_after_from_not_counted_in_select_list() {
    // A subquery in the FROM clause should not contribute to the SELECT list count
    let diags = check(
        "SELECT id, name FROM (SELECT id, name FROM customers) sub",
    );
    assert!(diags.is_empty(), "subquery in FROM clause should not be flagged");
}

#[test]
fn exactly_three_case_insensitive() {
    // Test that detection is case-insensitive for the SELECT keyword
    let diags = check(
        "select id, \
         (select count(*) from o where o.cid = c.id), \
         (select max(v) from o where o.cid = c.id), \
         (select min(v) from o where o.cid = c.id) \
         from c",
    );
    assert_eq!(diags.len(), 1, "detection should be case-insensitive");
}

#[test]
fn empty_sql_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}
