use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::subquery_in_select::SubqueryInSelect;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    SubqueryInSelect.check(&c)
}

// ── rule name ────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SubqueryInSelect.name(), "Structure/SubqueryInSelect");
}

// ── basic violation ──────────────────────────────────────────────────────────

#[test]
fn scalar_subquery_unnamed_in_select_list_is_violation() {
    let diags = check(
        "SELECT id, (SELECT name FROM c WHERE c.id = t.cat_id) FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn scalar_subquery_with_alias_in_select_list_is_violation() {
    let diags = check(
        "SELECT id, (SELECT name FROM c WHERE c.id = t.cat_id) AS cat FROM t",
    );
    assert_eq!(diags.len(), 1);
}

// ── no violation cases ───────────────────────────────────────────────────────

#[test]
fn plain_select_no_violation() {
    let diags = check("SELECT id, name FROM t");
    assert!(diags.is_empty());
}

#[test]
fn subquery_in_where_not_in_select_list_no_violation() {
    let diags = check(
        "SELECT id FROM t WHERE id IN (SELECT id FROM t2)",
    );
    assert!(diags.is_empty());
}

#[test]
fn subquery_in_from_no_violation() {
    let diags = check(
        "SELECT * FROM (SELECT * FROM t) sub",
    );
    assert!(diags.is_empty());
}

#[test]
fn subquery_in_join_no_violation() {
    let diags = check(
        "SELECT id FROM t JOIN (SELECT * FROM c) sub ON t.id = sub.id",
    );
    assert!(diags.is_empty());
}

// ── multiple violations ───────────────────────────────────────────────────────

#[test]
fn multiple_scalar_subqueries_in_select_list_multiple_violations() {
    let diags = check(
        "SELECT id, (SELECT name FROM c WHERE c.id = t.cat_id), (SELECT desc FROM d WHERE d.id = t.d_id) FROM t",
    );
    assert_eq!(diags.len(), 2);
}

// ── message format ────────────────────────────────────────────────────────────

#[test]
fn violation_message_mentions_n_plus_one() {
    let diags = check(
        "SELECT id, (SELECT name FROM c WHERE c.id = t.cat_id) AS cat FROM t",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "Scalar subquery in SELECT list may cause N+1 query performance issues; consider using a JOIN"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_empty() {
    let c = ctx("SELECTT INVALID GARBAGE @@##");
    if !c.parse_errors.is_empty() {
        let diags = SubqueryInSelect.check(&c);
        assert!(diags.is_empty());
    }
}

// ── CTE ───────────────────────────────────────────────────────────────────────

#[test]
fn cte_with_scalar_subquery_in_select_list_is_violation() {
    let diags = check(
        "WITH c AS (SELECT id, name FROM categories) SELECT id, (SELECT name FROM c WHERE c.id = t.cat_id) FROM t",
    );
    assert_eq!(diags.len(), 1);
}

// ── nested: outer and inner both have subquery-in-select ──────────────────────

#[test]
fn nested_subquery_in_select_both_detected() {
    // Outer SELECT has a scalar subquery item; that scalar subquery's own
    // SELECT list also has a scalar subquery item.
    let diags = check(
        "SELECT id, (SELECT (SELECT COUNT(*) FROM z) FROM c WHERE c.id = t.id) FROM t",
    );
    assert_eq!(diags.len(), 2);
}

// ── count aggregate as scalar subquery ───────────────────────────────────────

#[test]
fn count_aggregate_as_scalar_subquery_is_violation() {
    let diags = check("SELECT (SELECT COUNT(*) FROM t) AS total");
    assert_eq!(diags.len(), 1);
}

// ── rule field on diagnostic ──────────────────────────────────────────────────

#[test]
fn diagnostic_rule_field_is_correct() {
    let diags = check(
        "SELECT id, (SELECT name FROM c WHERE c.id = t.cat_id) AS cat FROM t",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/SubqueryInSelect");
}
