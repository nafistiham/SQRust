use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::order_by_in_subquery::OrderByInSubquery;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    OrderByInSubquery.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(OrderByInSubquery.name(), "OrderByInSubquery");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM ORDER BROKEN BY");
    assert!(diags.is_empty());
}

// ── top-level ORDER BY is exempt ──────────────────────────────────────────────

#[test]
fn top_level_order_by_no_violation() {
    let diags = check("SELECT * FROM t ORDER BY col");
    assert!(diags.is_empty());
}

// ── CTE with ORDER BY and no LIMIT → 1 violation ─────────────────────────────

#[test]
fn cte_with_order_by_no_limit_one_violation() {
    let diags = check(
        "WITH cte AS (SELECT id FROM t ORDER BY id) SELECT * FROM cte",
    );
    assert_eq!(diags.len(), 1);
}

// ── CTE with ORDER BY and LIMIT → 0 violations ───────────────────────────────

#[test]
fn cte_with_order_by_and_limit_no_violation() {
    let diags = check(
        "WITH cte AS (SELECT id FROM t ORDER BY id LIMIT 10) SELECT * FROM cte",
    );
    assert!(diags.is_empty());
}

// ── CTE with ORDER BY and OFFSET → 0 violations ───────────────────────────────

#[test]
fn cte_with_order_by_and_offset_no_violation() {
    let diags = check(
        "WITH cte AS (SELECT id FROM t ORDER BY id OFFSET 5) SELECT * FROM cte",
    );
    assert!(diags.is_empty());
}

// ── subquery in FROM with ORDER BY and no LIMIT → 1 violation ────────────────

#[test]
fn subquery_in_from_order_by_no_limit_one_violation() {
    let diags = check(
        "SELECT * FROM (SELECT id FROM t ORDER BY id) AS s",
    );
    assert_eq!(diags.len(), 1);
}

// ── subquery in FROM with ORDER BY and LIMIT → 0 violations ──────────────────

#[test]
fn subquery_in_from_order_by_with_limit_no_violation() {
    let diags = check(
        "SELECT * FROM (SELECT id FROM t ORDER BY id LIMIT 10) AS s",
    );
    assert!(diags.is_empty());
}

// ── subquery in WHERE with ORDER BY → 1 violation ────────────────────────────

#[test]
fn subquery_in_where_order_by_one_violation() {
    let diags = check(
        "SELECT * FROM t WHERE id IN (SELECT id FROM u ORDER BY id)",
    );
    assert_eq!(diags.len(), 1);
}

// ── no ORDER BY in subquery → 0 violations ───────────────────────────────────

#[test]
fn no_order_by_in_subquery_no_violation() {
    let diags = check(
        "SELECT * FROM (SELECT id FROM t) AS s",
    );
    assert!(diags.is_empty());
}

// ── multiple subqueries with ORDER BY → multiple violations ──────────────────

#[test]
fn multiple_subqueries_with_order_by_multiple_violations() {
    let diags = check(
        "SELECT * FROM (SELECT id FROM t ORDER BY id) AS a, (SELECT name FROM u ORDER BY name) AS b",
    );
    assert_eq!(diags.len(), 2);
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_is_nonzero() {
    let diags = check(
        "SELECT * FROM (SELECT id FROM t ORDER BY id) AS s",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── message format correct ────────────────────────────────────────────────────

#[test]
fn message_format_is_correct() {
    let diags = check(
        "SELECT * FROM (SELECT id FROM t ORDER BY id) AS s",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "ORDER BY in subquery without LIMIT has no effect on the final result"
    );
}

// ── subquery with LIMIT 10 is exempt (already tested above, extra alias) ──────

#[test]
fn subquery_with_limit_is_exempt() {
    let diags = check(
        "SELECT * FROM (SELECT id FROM t ORDER BY id LIMIT 10) AS s",
    );
    assert!(diags.is_empty());
}

// ── top-level query with ORDER BY, nested subquery without ORDER BY → 0 ───────

#[test]
fn top_level_order_by_nested_no_order_by_no_violation() {
    let diags = check(
        "SELECT * FROM (SELECT id FROM t) AS s ORDER BY id",
    );
    assert!(diags.is_empty());
}
