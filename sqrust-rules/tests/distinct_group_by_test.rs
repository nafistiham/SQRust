use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::distinct_group_by::DistinctGroupBy;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    DistinctGroupBy.check(&c)
}

// ── rule name ────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(DistinctGroupBy.name(), "Structure/DistinctGroupBy");
}

// ── basic violation ──────────────────────────────────────────────────────────

#[test]
fn distinct_with_group_by_one_column_is_violation() {
    let diags = check("SELECT DISTINCT col FROM t GROUP BY col");
    assert_eq!(diags.len(), 1);
}

#[test]
fn distinct_with_group_by_two_columns_is_violation() {
    let diags = check("SELECT DISTINCT col1, col2 FROM t GROUP BY col1, col2");
    assert_eq!(diags.len(), 1);
}

// ── no violation cases ───────────────────────────────────────────────────────

#[test]
fn distinct_without_group_by_no_violation() {
    let diags = check("SELECT DISTINCT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn group_by_without_distinct_no_violation() {
    let diags = check("SELECT col FROM t GROUP BY col");
    assert!(diags.is_empty());
}

#[test]
fn plain_select_no_violation() {
    let diags = check("SELECT col FROM t");
    assert!(diags.is_empty());
}

// ── case-insensitive ─────────────────────────────────────────────────────────

#[test]
fn lowercase_distinct_with_group_by_is_violation() {
    let diags = check("select distinct col from t group by col");
    assert_eq!(diags.len(), 1);
}

// ── subquery ─────────────────────────────────────────────────────────────────

#[test]
fn subquery_with_distinct_and_group_by_is_violation() {
    let diags = check(
        "SELECT * FROM (SELECT DISTINCT col FROM t GROUP BY col) sub",
    );
    assert_eq!(diags.len(), 1);
}

// ── UNION ─────────────────────────────────────────────────────────────────────

#[test]
fn union_branch_with_distinct_group_by_is_violation() {
    let diags = check(
        "SELECT col FROM t1 UNION ALL SELECT DISTINCT col FROM t2 GROUP BY col",
    );
    assert_eq!(diags.len(), 1);
}

// ── message format ────────────────────────────────────────────────────────────

#[test]
fn violation_message_mentions_redundant() {
    let diags = check("SELECT DISTINCT col FROM t GROUP BY col");
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "SELECT DISTINCT with GROUP BY is redundant; GROUP BY already deduplicates rows"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_empty() {
    let c = ctx("SELECTT INVALID GARBAGE @@##");
    if !c.parse_errors.is_empty() {
        let diags = DistinctGroupBy.check(&c);
        assert!(diags.is_empty());
    }
}

// ── aggregate in projection ───────────────────────────────────────────────────

#[test]
fn distinct_with_aggregate_and_group_by_is_violation() {
    let diags = check("SELECT DISTINCT COUNT(*) FROM t GROUP BY col");
    assert_eq!(diags.len(), 1);
}

// ── two queries, only one violates ───────────────────────────────────────────

#[test]
fn two_queries_only_one_has_distinct_group_by_one_violation() {
    let diags = check(
        "SELECT col FROM t1; SELECT DISTINCT col FROM t2 GROUP BY col",
    );
    assert_eq!(diags.len(), 1);
}

// ── rule field on diagnostic ──────────────────────────────────────────────────

#[test]
fn diagnostic_rule_field_is_correct() {
    let diags = check("SELECT DISTINCT col FROM t GROUP BY col");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/DistinctGroupBy");
}
