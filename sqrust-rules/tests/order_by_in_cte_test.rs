use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::order_by_in_cte::OrderByInCte;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    OrderByInCte.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(OrderByInCte.name(), "Structure/OrderByInCte");
}

// ── violation: ORDER BY inside a CTE ─────────────────────────────────────────

#[test]
fn order_by_in_cte_violation() {
    let diags = check(
        "WITH cte AS (SELECT a FROM t ORDER BY a) SELECT * FROM cte",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("cte"));
}

// ── no violation: ORDER BY only in outer query ────────────────────────────────

#[test]
fn no_order_by_in_cte_no_violation() {
    let diags = check(
        "WITH cte AS (SELECT a FROM t) SELECT * FROM cte ORDER BY a",
    );
    assert!(diags.is_empty());
}

// ── no violation: ORDER BY only in outer query (explicit) ─────────────────────

#[test]
fn order_by_in_outer_query_no_violation() {
    let diags = check(
        "SELECT a FROM t ORDER BY a",
    );
    assert!(diags.is_empty());
}

// ── one of two CTEs has ORDER BY ──────────────────────────────────────────────

#[test]
fn multiple_ctes_one_violation() {
    let diags = check(
        "WITH \
         cte1 AS (SELECT a FROM t ORDER BY a), \
         cte2 AS (SELECT b FROM u) \
         SELECT * FROM cte1 JOIN cte2 ON cte1.a = cte2.b",
    );
    assert_eq!(diags.len(), 1);
}

// ── both CTEs have ORDER BY ───────────────────────────────────────────────────

#[test]
fn multiple_ctes_both_violation() {
    let diags = check(
        "WITH \
         cte1 AS (SELECT a FROM t ORDER BY a), \
         cte2 AS (SELECT b FROM u ORDER BY b) \
         SELECT * FROM cte1 JOIN cte2 ON cte1.a = cte2.b",
    );
    assert_eq!(diags.len(), 2);
}

// ── no CTE at all ─────────────────────────────────────────────────────────────

#[test]
fn no_cte_no_violation() {
    let diags = check("SELECT a, b FROM t WHERE a > 1");
    assert!(diags.is_empty());
}

// ── empty file ────────────────────────────────────────────────────────────────

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

// ── parse error returns no violations ─────────────────────────────────────────

#[test]
fn parse_error_no_violation() {
    let c = ctx("WITH BROKEN AS SELECT ORDER ORDER BY");
    if !c.parse_errors.is_empty() {
        let diags = OrderByInCte.check(&c);
        assert!(diags.is_empty());
    }
}

// ── CTE with both LIMIT and ORDER BY — still a violation ──────────────────────

#[test]
fn cte_with_limit_and_order_by_violation() {
    // Even with LIMIT, ORDER BY in a CTE still has no guaranteed effect on the
    // outer query's result set ordering. We flag it.
    let diags = check(
        "WITH cte AS (SELECT a FROM t ORDER BY a LIMIT 10) SELECT * FROM cte",
    );
    assert_eq!(diags.len(), 1);
}

// ── nested CTE (CTE inside a CTE body) ───────────────────────────────────────

#[test]
fn nested_cte_violation() {
    let diags = check(
        "WITH outer_cte AS (\
           WITH inner_cte AS (SELECT a FROM t ORDER BY a) \
           SELECT * FROM inner_cte\
         ) SELECT * FROM outer_cte",
    );
    // At least the inner CTE ORDER BY should be flagged.
    assert!(!diags.is_empty());
}

// ── subquery ORDER BY in FROM clause is NOT a CTE — should not be flagged here ─

#[test]
fn order_by_in_subquery_not_in_cte_no_violation() {
    // This is a derived table / subquery, not a CTE.
    // Structure/OrderByInSubquery handles this; OrderByInCte should not flag it.
    let diags = check(
        "SELECT * FROM (SELECT a FROM t ORDER BY a) AS sub",
    );
    assert!(diags.is_empty());
}

// ── two CTEs, neither has ORDER BY ───────────────────────────────────────────

#[test]
fn two_ctes_no_order_by_no_violation() {
    let diags = check(
        "WITH \
         cte1 AS (SELECT a FROM t), \
         cte2 AS (SELECT b FROM u) \
         SELECT * FROM cte1 JOIN cte2 ON cte1.a = cte2.b",
    );
    assert!(diags.is_empty());
}

// ── message format ────────────────────────────────────────────────────────────

#[test]
fn message_contains_cte_name() {
    let diags = check(
        "WITH my_cte AS (SELECT a FROM t ORDER BY a) SELECT * FROM my_cte",
    );
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("my_cte"),
        "expected message to contain CTE name, got: {}",
        diags[0].message
    );
}
