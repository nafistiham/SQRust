use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::nested_subquery::NestedSubquery;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    NestedSubquery::default().check(&c)
}

fn check_with(sql: &str, max_depth: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    NestedSubquery { max_depth }.check(&c)
}

// ── rule name ────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(NestedSubquery::default().name(), "Structure/NestedSubquery");
}

// ── default max_depth ────────────────────────────────────────────────────────

#[test]
fn default_max_depth_is_two() {
    assert_eq!(NestedSubquery::default().max_depth, 2);
}

// ── no subquery ──────────────────────────────────────────────────────────────

#[test]
fn plain_select_no_violation() {
    let diags = check("SELECT col FROM t");
    assert!(diags.is_empty());
}

// ── single level (depth 1) ───────────────────────────────────────────────────

#[test]
fn one_subquery_no_violation() {
    let diags = check("SELECT * FROM (SELECT * FROM t) sub");
    assert!(diags.is_empty());
}

// ── two levels (depth 2, at max) ─────────────────────────────────────────────

#[test]
fn two_subqueries_at_max_no_violation() {
    let diags = check(
        "SELECT * FROM (SELECT * FROM (SELECT * FROM t) s1) s2",
    );
    assert!(diags.is_empty());
}

// ── three levels (depth 3, over max) ─────────────────────────────────────────

#[test]
fn three_subqueries_over_max_one_violation() {
    let diags = check(
        "SELECT * FROM (SELECT * FROM (SELECT * FROM (SELECT * FROM t) s1) s2) s3",
    );
    assert_eq!(diags.len(), 1);
}

// ── message content ───────────────────────────────────────────────────────────

#[test]
fn violation_message_contains_depth_and_max() {
    let diags = check(
        "SELECT * FROM (SELECT * FROM (SELECT * FROM (SELECT * FROM t) s1) s2) s3",
    );
    assert_eq!(diags.len(), 1);
    // depth = 3, max = 2
    assert!(diags[0].message.contains('3'), "message should contain the depth count");
    assert!(diags[0].message.contains('2'), "message should contain the max");
    assert!(diags[0].message.contains("CTE") || diags[0].message.contains("cte"),
        "message should mention CTEs as the suggested alternative");
}

// ── custom max_depth ─────────────────────────────────────────────────────────

#[test]
fn custom_max_depth_one_two_subqueries_is_violation() {
    // max_depth = 1 → 2 subqueries triggers
    let diags = check_with(
        "SELECT * FROM (SELECT * FROM (SELECT * FROM t) s1) s2",
        1,
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn custom_max_depth_one_single_subquery_no_violation() {
    let diags = check_with("SELECT * FROM (SELECT * FROM t) sub", 1);
    assert!(diags.is_empty());
}

#[test]
fn custom_max_depth_three_three_subqueries_no_violation() {
    let diags = check_with(
        "SELECT * FROM (SELECT * FROM (SELECT * FROM (SELECT * FROM t) s1) s2) s3",
        3,
    );
    assert!(diags.is_empty());
}

// ── skip map: comment ────────────────────────────────────────────────────────

#[test]
fn select_in_line_comment_not_counted() {
    // The comment contains '(SELECT' but it must not be counted
    let diags = check(
        "SELECT col FROM t -- (SELECT * FROM (SELECT * FROM (SELECT x FROM z) a) b)",
    );
    assert!(diags.is_empty());
}

// ── skip map: string literal ─────────────────────────────────────────────────

#[test]
fn select_in_string_literal_not_counted() {
    let diags = check(
        "SELECT '(SELECT * FROM (SELECT * FROM (SELECT x FROM z) a) b)' FROM t",
    );
    assert!(diags.is_empty());
}

// ── violation is flagged exactly once ────────────────────────────────────────

#[test]
fn exactly_one_diagnostic_for_three_deep_nesting() {
    let diags = check(
        "SELECT * FROM (SELECT * FROM (SELECT * FROM (SELECT * FROM t) s1) s2) s3",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/NestedSubquery");
}
