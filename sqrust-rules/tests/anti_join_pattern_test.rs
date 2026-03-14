use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::anti_join_pattern::AntiJoinPattern;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    AntiJoinPattern.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(AntiJoinPattern.name(), "Structure/AntiJoinPattern");
}

#[test]
fn not_in_subquery_flagged() {
    let diags = check(
        "SELECT * FROM orders WHERE customer_id NOT IN (SELECT customer_id FROM vip_customers)",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/AntiJoinPattern");
}

#[test]
fn not_in_literal_list_no_violation() {
    // NOT IN with a literal list, not a subquery — should not be flagged
    let diags = check("SELECT * FROM orders WHERE status NOT IN ('cancelled', 'refunded')");
    assert!(diags.is_empty());
}

#[test]
fn in_subquery_without_not_no_violation() {
    let diags = check(
        "SELECT * FROM orders WHERE customer_id IN (SELECT customer_id FROM vip_customers)",
    );
    assert!(diags.is_empty());
}

#[test]
fn simple_where_no_violation() {
    let diags = check("SELECT * FROM orders WHERE status = 'active'");
    assert!(diags.is_empty());
}

#[test]
fn not_in_subquery_case_insensitive_lower() {
    let diags = check(
        "select * from orders where customer_id not in (select customer_id from vip_customers)",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_in_subquery_case_insensitive_mixed() {
    let diags = check(
        "SELECT * FROM orders WHERE customer_id NOT IN (SELECT customer_id FROM vip_customers)",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_in_subquery_multiline_flagged() {
    let sql = "SELECT *\nFROM orders\nWHERE customer_id NOT IN (\n    SELECT customer_id\n    FROM vip_customers\n)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_mentions_null_or_not_exists() {
    let diags = check(
        "SELECT * FROM orders WHERE customer_id NOT IN (SELECT customer_id FROM vip_customers)",
    );
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("null") || msg.contains("not exists") || msg.contains("left join"),
        "expected message to mention NULL safety or alternatives, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check(
        "SELECT * FROM orders WHERE customer_id NOT IN (SELECT customer_id FROM vip_customers)",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line must be >= 1");
    assert!(diags[0].col >= 1, "col must be >= 1");
}

#[test]
fn not_in_subquery_in_string_not_flagged() {
    // NOT IN (SELECT inside a string literal should not be flagged
    let diags = check("SELECT 'NOT IN (SELECT ...) is bad' AS tip FROM dual");
    assert!(diags.is_empty());
}

#[test]
fn not_in_subquery_in_comment_not_flagged() {
    // NOT IN (SELECT inside a comment should not be flagged
    let diags = check("SELECT * FROM t -- NOT IN (SELECT bad)\nWHERE x = 1");
    assert!(diags.is_empty());
}

#[test]
fn multiple_not_in_subqueries_multiple_violations() {
    let sql = "SELECT * FROM t \
               WHERE a NOT IN (SELECT a FROM a_table) \
               AND b NOT IN (SELECT b FROM b_table)";
    let diags = check(sql);
    assert_eq!(diags.len(), 2, "each NOT IN (SELECT ...) should produce one violation");
}

#[test]
fn not_exists_no_violation() {
    // NOT EXISTS is the preferred pattern — should not be flagged
    let diags = check(
        "SELECT * FROM orders o WHERE NOT EXISTS (SELECT 1 FROM vip_customers v WHERE v.customer_id = o.customer_id)",
    );
    assert!(diags.is_empty());
}

#[test]
fn not_in_subquery_in_cte_flagged() {
    let sql = "
        WITH excluded AS (
            SELECT id FROM orders WHERE status NOT IN (SELECT status FROM active_statuses)
        )
        SELECT * FROM excluded
    ";
    let diags = check(sql);
    assert_eq!(diags.len(), 1, "NOT IN (SELECT) inside a CTE should be flagged");
}
