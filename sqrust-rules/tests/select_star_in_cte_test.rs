use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::select_star_in_cte::SelectStarInCTE;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    SelectStarInCTE.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelectStarInCTE.name(), "Structure/SelectStarInCTE");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_no_violations() {
    let diags = check("SELECT FROM FROM WHERE");
    assert!(diags.is_empty());
}

// ── SELECT * in CTE body — 1 violation ───────────────────────────────────────

#[test]
fn select_star_in_cte_violation() {
    let diags = check("WITH c AS (SELECT * FROM t) SELECT col FROM c");
    assert_eq!(diags.len(), 1);
}

// ── explicit columns in CTE — no violation ───────────────────────────────────

#[test]
fn select_explicit_in_cte_no_violation() {
    let diags = check("WITH c AS (SELECT a, b FROM t) SELECT a FROM c");
    assert!(diags.is_empty());
}

// ── SELECT * in main query, not in CTE body — no violation ───────────────────

#[test]
fn select_star_in_main_query_no_violation() {
    let diags = check("WITH c AS (SELECT a FROM t) SELECT * FROM c");
    assert!(diags.is_empty());
}

// ── two CTEs, one with star — 1 violation ────────────────────────────────────

#[test]
fn two_ctes_one_with_star_one_violation() {
    let diags = check(
        "WITH c1 AS (SELECT * FROM t), c2 AS (SELECT id FROM t) SELECT * FROM c1",
    );
    assert_eq!(diags.len(), 1);
}

// ── two CTEs, both with star — 2 violations ──────────────────────────────────

#[test]
fn two_ctes_both_with_star_two_violations() {
    let diags = check(
        "WITH c1 AS (SELECT * FROM t1), c2 AS (SELECT * FROM t2) SELECT a FROM c1",
    );
    assert_eq!(diags.len(), 2);
}

// ── CTE name appears in message ───────────────────────────────────────────────

#[test]
fn cte_name_in_message() {
    let diags = check("WITH my_cte AS (SELECT * FROM t) SELECT col FROM my_cte");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("my_cte"),
        "expected message to mention CTE name 'my_cte', got: {}",
        diags[0].message
    );
}

// ── message_mentions_cte_name (alias for above) ───────────────────────────────

#[test]
fn message_mentions_cte_name() {
    let diags = check("WITH orders AS (SELECT * FROM raw_orders) SELECT id FROM orders");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("orders"),
        "expected message to contain 'orders', got: {}",
        diags[0].message
    );
}

// ── no CTE — no violation ─────────────────────────────────────────────────────

#[test]
fn no_cte_no_violation() {
    let diags = check("SELECT * FROM t WHERE id = 1");
    assert!(diags.is_empty());
}

// ── line and col are nonzero ──────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("WITH c AS (SELECT * FROM t) SELECT col FROM c");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── qualified wildcard (t.*) inside CTE — 1 violation ────────────────────────

#[test]
fn select_qualified_star_in_cte_violation() {
    let diags = check("WITH c AS (SELECT t.* FROM t) SELECT col FROM c");
    assert_eq!(diags.len(), 1);
}

// ── nested subquery with star inside a CTE — 1 violation ─────────────────────

#[test]
fn nested_subquery_star_in_cte_violation() {
    let diags = check(
        "WITH c AS (SELECT x FROM (SELECT * FROM t) sub) SELECT x FROM c",
    );
    assert_eq!(diags.len(), 1);
}

// ── CTE with UNION — star in one union branch inside CTE ─────────────────────

#[test]
fn cte_union_branch_star_violation() {
    let diags = check(
        "WITH c AS (SELECT * FROM t1 UNION ALL SELECT id FROM t2) SELECT id FROM c",
    );
    assert_eq!(diags.len(), 1);
}
