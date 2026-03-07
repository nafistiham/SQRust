use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::too_many_subqueries::TooManySubqueries;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    TooManySubqueries::default().check(&c)
}

fn check_with(sql: &str, max_subqueries: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    TooManySubqueries { max_subqueries }.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        TooManySubqueries::default().name(),
        "Structure/TooManySubqueries"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM WHERE BROKEN AND");
    assert!(diags.is_empty());
}

// ── 1 subquery — default max=3 — no violation ────────────────────────────────

#[test]
fn one_subquery_default_max_no_violation() {
    let sql = "SELECT * FROM t WHERE id IN (SELECT id FROM other)";
    let diags = check(sql);
    assert!(diags.is_empty(), "1 subquery should not trigger at max=3");
}

// ── 3 subqueries at default max=3 — no violation ─────────────────────────────

#[test]
fn three_subqueries_at_default_max_no_violation() {
    // Three scalar subqueries in SELECT list
    let sql = "SELECT (SELECT 1), (SELECT 2), (SELECT 3) FROM t";
    let diags = check(sql);
    assert!(
        diags.is_empty(),
        "3 subqueries at max=3 should not trigger"
    );
}

// ── 4 subqueries over default max=3 — 1 violation ────────────────────────────

#[test]
fn four_subqueries_over_default_max_one_violation() {
    let sql = "SELECT (SELECT 1), (SELECT 2), (SELECT 3), (SELECT 4) FROM t";
    let diags = check(sql);
    assert_eq!(
        diags.len(),
        1,
        "4 subqueries over max=3 should produce 1 violation"
    );
}

// ── no subqueries — no violation ─────────────────────────────────────────────

#[test]
fn no_subquery_no_violation() {
    let diags = check("SELECT id, name FROM t WHERE active = 1");
    assert!(diags.is_empty());
}

// ── custom max=1: 2 subqueries — 1 violation ─────────────────────────────────

#[test]
fn custom_max_1_two_subqueries_one_violation() {
    let sql = "SELECT (SELECT 1), (SELECT 2) FROM t";
    let diags = check_with(sql, 1);
    assert_eq!(diags.len(), 1);
}

// ── custom max=1: 1 subquery — no violation ───────────────────────────────────

#[test]
fn custom_max_1_one_subquery_no_violation() {
    let sql = "SELECT * FROM t WHERE id IN (SELECT id FROM other)";
    let diags = check_with(sql, 1);
    assert!(diags.is_empty());
}

// ── default max is 3 ──────────────────────────────────────────────────────────

#[test]
fn default_max_is_three() {
    assert_eq!(TooManySubqueries::default().max_subqueries, 3);
}

// ── message contains count and max ───────────────────────────────────────────

#[test]
fn message_contains_count_and_max() {
    let sql = "SELECT (SELECT 1), (SELECT 2), (SELECT 3), (SELECT 4) FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('4'),
        "message should contain the subquery count (4): got '{}'",
        diags[0].message
    );
    assert!(
        diags[0].message.contains('3'),
        "message should contain the max (3): got '{}'",
        diags[0].message
    );
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let sql = "SELECT (SELECT 1), (SELECT 2), (SELECT 3), (SELECT 4) FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── EXISTS counted ───────────────────────────────────────────────────────────

#[test]
fn exists_subquery_counted() {
    // 3 EXISTS subqueries + 1 scalar = 4 total > max=3 → flag
    let sql = "SELECT * FROM t \
               WHERE EXISTS (SELECT 1 FROM a) \
               AND EXISTS (SELECT 1 FROM b) \
               AND EXISTS (SELECT 1 FROM c) \
               AND EXISTS (SELECT 1 FROM d)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1, "EXISTS subqueries should be counted");
}

// ── IN subquery counted ───────────────────────────────────────────────────────

#[test]
fn in_subquery_counted() {
    // 4 IN subqueries > max=3 → flag
    let sql = "SELECT * FROM t \
               WHERE a IN (SELECT a FROM x) \
               AND b IN (SELECT b FROM y) \
               AND c IN (SELECT c FROM z) \
               AND d IN (SELECT d FROM w)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1, "IN subqueries should be counted");
}

// ── correlated subquery counted ───────────────────────────────────────────────

#[test]
fn correlated_subquery_counted() {
    // Correlated scalar subqueries in SELECT list — 4 > max=3 → flag
    let sql = "SELECT \
               (SELECT MAX(salary) FROM emp e2 WHERE e2.dept_id = e.dept_id), \
               (SELECT MIN(salary) FROM emp e2 WHERE e2.dept_id = e.dept_id), \
               (SELECT AVG(salary) FROM emp e2 WHERE e2.dept_id = e.dept_id), \
               (SELECT COUNT(*) FROM emp e2 WHERE e2.dept_id = e.dept_id) \
               FROM emp e";
    let diags = check(sql);
    assert_eq!(diags.len(), 1, "Correlated subqueries should be counted");
}
