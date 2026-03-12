use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::deep_cte_chain::DeepCteChain;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    DeepCteChain::default().check(&c)
}

fn check_with(sql: &str, max_depth: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    DeepCteChain { max_depth }.check(&c)
}

// ── rule name ────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(DeepCteChain::default().name(), "Structure/DeepCteChain");
}

// ── default max_depth ────────────────────────────────────────────────────────

#[test]
fn default_max_depth_is_five() {
    assert_eq!(DeepCteChain::default().max_depth, 5);
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("WITH AS SELECT BROKEN FROM");
    assert!(diags.is_empty());
}

// ── no WITH clause ────────────────────────────────────────────────────────────

#[test]
fn no_with_clause_no_violation() {
    let diags = check("SELECT 1");
    assert!(diags.is_empty());
}

// ── single CTE — depth 1 — no violation ──────────────────────────────────────

#[test]
fn single_cte_no_violation() {
    let diags = check("WITH a AS (SELECT 1) SELECT * FROM a");
    assert!(diags.is_empty());
}

// ── 3 independent CTEs — depth 1 for all — no violation ──────────────────────

#[test]
fn three_independent_ctes_no_violation() {
    let diags = check(
        "WITH a AS (SELECT 1), \
         b AS (SELECT 2), \
         c AS (SELECT 3) \
         SELECT * FROM a",
    );
    assert!(diags.is_empty());
}

// ── chain of 3: C→B→A — depth 3 — default max 5 — no violation ───────────────

#[test]
fn chain_of_3_default_max_no_violation() {
    // a: depth 1, b refs a: depth 2, c refs b: depth 3
    let diags = check(
        "WITH a AS (SELECT 1), \
         b AS (SELECT id FROM a), \
         c AS (SELECT id FROM b) \
         SELECT * FROM c",
    );
    assert!(diags.is_empty());
}

// ── chain of 5 — depth 5 — at limit — no violation ───────────────────────────

#[test]
fn chain_of_5_at_default_max_no_violation() {
    let diags = check(
        "WITH a AS (SELECT 1), \
         b AS (SELECT id FROM a), \
         c AS (SELECT id FROM b), \
         d AS (SELECT id FROM c), \
         e AS (SELECT id FROM d) \
         SELECT * FROM e",
    );
    assert!(diags.is_empty());
}

// ── chain of 6 — depth 6 — default max 5 — 1 violation ───────────────────────

#[test]
fn chain_of_6_default_max_one_violation() {
    let diags = check(
        "WITH a AS (SELECT 1), \
         b AS (SELECT id FROM a), \
         c AS (SELECT id FROM b), \
         d AS (SELECT id FROM c), \
         e AS (SELECT id FROM d), \
         f AS (SELECT id FROM e) \
         SELECT * FROM f",
    );
    assert_eq!(diags.len(), 1);
}

// ── custom max_depth=3: chain of 4 → 1 violation ─────────────────────────────

#[test]
fn custom_max_3_chain_of_4_one_violation() {
    let diags = check_with(
        "WITH a AS (SELECT 1), \
         b AS (SELECT id FROM a), \
         c AS (SELECT id FROM b), \
         d AS (SELECT id FROM c) \
         SELECT * FROM d",
        3,
    );
    assert_eq!(diags.len(), 1);
}

// ── custom max_depth=3: chain of 3 → 0 violations ────────────────────────────

#[test]
fn custom_max_3_chain_of_3_no_violation() {
    let diags = check_with(
        "WITH a AS (SELECT 1), \
         b AS (SELECT id FROM a), \
         c AS (SELECT id FROM b) \
         SELECT * FROM c",
        3,
    );
    assert!(diags.is_empty());
}

// ── custom max_depth=2: chain of 3 → 1 violation ─────────────────────────────

#[test]
fn custom_max_2_chain_of_3_one_violation() {
    let diags = check_with(
        "WITH a AS (SELECT 1), \
         b AS (SELECT id FROM a), \
         c AS (SELECT id FROM b) \
         SELECT * FROM c",
        2,
    );
    assert_eq!(diags.len(), 1);
}

// ── diamond: A, B refs A, C refs A, D refs B and C — max depth 3 — no violation

#[test]
fn diamond_shape_depth_3_default_max_no_violation() {
    // a: depth 1, b refs a: depth 2, c refs a: depth 2, d refs b and c: depth 3
    let diags = check(
        "WITH a AS (SELECT 1), \
         b AS (SELECT id FROM a), \
         c AS (SELECT val FROM a), \
         d AS (SELECT b.id, c.val FROM b JOIN c ON b.id = c.val) \
         SELECT * FROM d",
    );
    assert!(diags.is_empty());
}

// ── message contains depth and max ───────────────────────────────────────────

#[test]
fn message_contains_depth_and_max() {
    let diags = check(
        "WITH a AS (SELECT 1), \
         b AS (SELECT id FROM a), \
         c AS (SELECT id FROM b), \
         d AS (SELECT id FROM c), \
         e AS (SELECT id FROM d), \
         f AS (SELECT id FROM e) \
         SELECT * FROM f",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains('6'), "message should contain depth 6");
    assert!(diags[0].message.contains('5'), "message should contain max 5");
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_is_nonzero() {
    let diags = check(
        "WITH a AS (SELECT 1), \
         b AS (SELECT id FROM a), \
         c AS (SELECT id FROM b), \
         d AS (SELECT id FROM c), \
         e AS (SELECT id FROM d), \
         f AS (SELECT id FROM e) \
         SELECT * FROM f",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── rule name in diagnostic ───────────────────────────────────────────────────

#[test]
fn diagnostic_has_correct_rule_name() {
    let diags = check(
        "WITH a AS (SELECT 1), \
         b AS (SELECT id FROM a), \
         c AS (SELECT id FROM b), \
         d AS (SELECT id FROM c), \
         e AS (SELECT id FROM d), \
         f AS (SELECT id FROM e) \
         SELECT * FROM f",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/DeepCteChain");
}
