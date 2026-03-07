use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::exists_over_in::ExistsOverIn;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ExistsOverIn.check(&ctx)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(ExistsOverIn.name(), "Convention/ExistsOverIn");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = ExistsOverIn.check(&ctx);
        assert!(diags.is_empty());
    }
}

// ── basic violation ───────────────────────────────────────────────────────────

#[test]
fn in_subquery_one_violation() {
    let diags = check("SELECT id FROM t WHERE id IN (SELECT id FROM u)");
    assert_eq!(diags.len(), 1);
}

// ── no violation cases ────────────────────────────────────────────────────────

#[test]
fn not_in_subquery_no_violation() {
    let diags = check("SELECT id FROM t WHERE id NOT IN (SELECT id FROM u)");
    assert!(diags.is_empty());
}

#[test]
fn in_literal_list_no_violation() {
    let diags = check("SELECT id FROM t WHERE id IN (1, 2, 3)");
    assert!(diags.is_empty());
}

#[test]
fn select_without_in_no_violation() {
    let diags = check("SELECT id FROM t WHERE id = 1");
    assert!(diags.is_empty());
}

// ── multiple violations ───────────────────────────────────────────────────────

#[test]
fn two_in_subqueries_two_violations() {
    let diags = check(
        "SELECT id FROM t WHERE id IN (SELECT id FROM u) AND name IN (SELECT name FROM v)",
    );
    assert_eq!(diags.len(), 2);
}

// ── projection violation ──────────────────────────────────────────────────────

#[test]
fn in_subquery_in_select_projection_violation() {
    // Scalar IN-subquery appearing directly in the SELECT list
    let diags = check(
        "SELECT (id IN (SELECT id FROM u)) AS flag FROM t",
    );
    assert_eq!(diags.len(), 1);
}

// ── HAVING violation ──────────────────────────────────────────────────────────

#[test]
fn in_subquery_in_having_violation() {
    let diags = check(
        "SELECT dept_id, COUNT(*) FROM t GROUP BY dept_id HAVING dept_id IN (SELECT id FROM d)",
    );
    assert_eq!(diags.len(), 1);
}

// ── message content ───────────────────────────────────────────────────────────

#[test]
fn message_contains_exists() {
    let diags = check("SELECT id FROM t WHERE id IN (SELECT id FROM u)");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_uppercase().contains("EXISTS"),
        "message should mention EXISTS, got: {}",
        diags[0].message
    );
}

// ── position ──────────────────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT id FROM t WHERE id IN (SELECT id FROM u)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── CTE — flag only the IN use, not the CTE definition ───────────────────────

#[test]
fn in_with_cte_no_flag_of_cte_itself() {
    // The CTE itself is just a named SELECT — not an IN (SELECT ...) pattern.
    // The outer WHERE uses IN (SELECT ...) and should be flagged once.
    let sql = "WITH cte AS (SELECT id FROM u) SELECT id FROM t WHERE id IN (SELECT id FROM cte)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

// ── nested IN subquery ────────────────────────────────────────────────────────

#[test]
fn nested_in_subquery_violation() {
    // The outer IN (SELECT ...) from u should be flagged.
    // The inner correlated subquery inside u's WHERE is also IN (SELECT ...) — flagged.
    let sql =
        "SELECT id FROM t WHERE id IN (SELECT id FROM u WHERE u.x IN (SELECT x FROM v))";
    let diags = check(sql);
    // Two IN (SELECT ...) patterns — both flagged
    assert_eq!(diags.len(), 2);
}
