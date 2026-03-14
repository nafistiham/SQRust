use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::having_without_select_agg::HavingWithoutSelectAgg;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    HavingWithoutSelectAgg.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        HavingWithoutSelectAgg.name(),
        "Structure/HavingWithoutSelectAgg"
    );
}

// ── HAVING COUNT(*) but no agg in SELECT — violation ─────────────────────────

#[test]
fn having_count_no_select_agg_violation() {
    let diags = check("SELECT a FROM t GROUP BY a HAVING COUNT(*) > 5");
    assert_eq!(diags.len(), 1);
}

// ── HAVING COUNT(*) and COUNT(*) in SELECT — no violation ────────────────────

#[test]
fn having_count_with_select_count_no_violation() {
    let diags = check("SELECT a, COUNT(*) FROM t GROUP BY a HAVING COUNT(*) > 5");
    assert!(diags.is_empty());
}

// ── no HAVING — no violation ──────────────────────────────────────────────────

#[test]
fn no_having_no_violation() {
    let diags = check("SELECT a FROM t GROUP BY a");
    assert!(diags.is_empty());
}

// ── HAVING with non-aggregate condition — no violation ────────────────────────

#[test]
fn having_with_non_aggregate_no_violation() {
    let diags = check("SELECT a FROM t GROUP BY a HAVING a > 5");
    assert!(diags.is_empty());
}

// ── HAVING SUM but no agg in SELECT — violation ───────────────────────────────

#[test]
fn having_sum_no_select_agg_violation() {
    let diags = check("SELECT a FROM t GROUP BY a HAVING SUM(val) > 100");
    assert_eq!(diags.len(), 1);
}

// ── HAVING MAX with SELECT MAX — no violation ─────────────────────────────────

#[test]
fn having_max_with_select_max_no_violation() {
    let diags = check("SELECT a, MAX(val) FROM t GROUP BY a HAVING MAX(val) > 50");
    assert!(diags.is_empty());
}

// ── message content ───────────────────────────────────────────────────────────

#[test]
fn message_content() {
    let diags = check("SELECT a FROM t GROUP BY a HAVING COUNT(*) > 5");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("having") || msg.contains("aggregate") || msg.contains("select"),
        "expected message to mention HAVING/aggregate/SELECT, got: {}",
        diags[0].message
    );
}

// ── parse error — no violations ───────────────────────────────────────────────

#[test]
fn parse_error_no_violations() {
    let diags = check("SELECT FROM FROM WHERE");
    assert!(diags.is_empty());
}

// ── line and col are nonzero ──────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT a FROM t GROUP BY a HAVING COUNT(*) > 5");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── SELECT has different agg than HAVING — no violation ───────────────────────
// (SELECT has some agg, so we don't flag)

#[test]
fn having_and_select_different_aggs_no_violation() {
    let diags = check("SELECT a, SUM(val) FROM t GROUP BY a HAVING COUNT(*) > 1");
    assert!(diags.is_empty());
}

// ── CTE with HAVING without SELECT agg — violation ───────────────────────────

#[test]
fn cte_with_having_without_select_agg_violation() {
    let diags = check(
        "WITH cte AS (SELECT a FROM t GROUP BY a HAVING COUNT(*) > 3) SELECT * FROM cte",
    );
    assert_eq!(diags.len(), 1);
}

// ── subquery with HAVING without SELECT agg — violation ───────────────────────

#[test]
fn subquery_having_without_select_agg_violation() {
    let diags = check(
        "SELECT x FROM (SELECT a FROM t GROUP BY a HAVING SUM(val) > 100) sub",
    );
    assert_eq!(diags.len(), 1);
}
