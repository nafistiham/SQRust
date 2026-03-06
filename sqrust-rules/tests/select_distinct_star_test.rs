use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::select_distinct_star::SelectDistinctStar;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SelectDistinctStar.check(&ctx)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelectDistinctStar.name(), "SelectDistinctStar");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = SelectDistinctStar.check(&ctx);
        assert!(diags.is_empty());
    }
}

// ── basic violation ───────────────────────────────────────────────────────────

#[test]
fn select_distinct_star_one_violation() {
    let diags = check("SELECT DISTINCT * FROM t");
    assert_eq!(diags.len(), 1);
}

// ── no violation cases ────────────────────────────────────────────────────────

#[test]
fn select_distinct_column_no_violation() {
    let diags = check("SELECT DISTINCT id FROM t");
    assert!(diags.is_empty());
}

#[test]
fn select_star_no_distinct_no_violation() {
    let diags = check("SELECT * FROM t");
    assert!(diags.is_empty());
}

#[test]
fn select_distinct_multiple_columns_no_violation() {
    let diags = check("SELECT DISTINCT a, b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_select_no_violation() {
    let diags = check("UPDATE t SET a = 1");
    assert!(diags.is_empty());
}

// ── qualified wildcard ────────────────────────────────────────────────────────

#[test]
fn select_distinct_qualified_star_violation() {
    // SELECT DISTINCT t.* FROM t — QualifiedWildcard should also be flagged
    let diags = check("SELECT DISTINCT t.* FROM t");
    assert_eq!(diags.len(), 1);
}

// ── subquery ─────────────────────────────────────────────────────────────────

#[test]
fn subquery_select_distinct_star_violation() {
    let diags = check("SELECT (SELECT DISTINCT * FROM t) FROM dual");
    assert_eq!(diags.len(), 1);
}

// ── message and position ──────────────────────────────────────────────────────

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT DISTINCT * FROM t");
    assert_eq!(diags.len(), 1);
    assert!(!diags[0].message.is_empty());
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT DISTINCT * FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── multiple tables / join ────────────────────────────────────────────────────

#[test]
fn select_distinct_star_multiple_tables_violation() {
    let diags = check("SELECT DISTINCT * FROM t JOIN u ON t.id = u.id");
    assert_eq!(diags.len(), 1);
}

// ── CTE ───────────────────────────────────────────────────────────────────────

#[test]
fn cte_with_select_distinct_star_violation() {
    // The DISTINCT * is inside the CTE body — should be flagged
    let diags =
        check("WITH cte AS (SELECT DISTINCT * FROM t) SELECT * FROM cte");
    assert_eq!(diags.len(), 1);
}
