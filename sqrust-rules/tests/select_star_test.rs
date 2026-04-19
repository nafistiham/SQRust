use sqrust_core::FileContext;
use sqrust_rules::convention::select_star::SelectStar;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SelectStar.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelectStar.name(), "Convention/SelectStar");
}

#[test]
fn select_star_flagged() {
    let diags = check("SELECT * FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
}

#[test]
fn qualified_wildcard_flagged() {
    let diags = check("SELECT t.* FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
}

#[test]
fn explicit_columns_no_violation() {
    let diags = check("SELECT a, b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn count_star_no_violation() {
    // COUNT(*) — must not be flagged
    let diags = check("SELECT COUNT(*) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn arithmetic_no_false_positive() {
    // AST rule: a * b is not a wildcard — must not be flagged
    let diags = check("SELECT a * b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn star_inside_string_no_violation() {
    let diags = check("SELECT '* is a star' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn star_inside_line_comment_no_violation() {
    let diags = check("SELECT 1 -- * is a star");
    assert!(diags.is_empty());
}

#[test]
fn star_inside_block_comment_no_violation() {
    let diags = check("SELECT 1 /* * is a star */");
    assert!(diags.is_empty());
}

#[test]
fn select_star_comma_a_one_violation() {
    // "SELECT *, a FROM t" — one wildcard
    let diags = check("SELECT *, a FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_star_on_line_three_correct_line_number() {
    let sql = "SELECT a\nFROM t\nWHERE x IN (SELECT * FROM u)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}

#[test]
fn select_star_violation_has_correct_message() {
    let diags = check("SELECT * FROM t");
    assert_eq!(diags[0].message, "Avoid SELECT *; list columns explicitly");
}

#[test]
fn nested_subquery_select_star_flagged() {
    let diags = check("SELECT id FROM t WHERE id IN (SELECT * FROM u)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cte_select_star_flagged() {
    let diags = check("WITH cte AS (SELECT * FROM t) SELECT id FROM cte");
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_skipped_gracefully() {
    // Unparseable SQL should produce no violations (not a crash)
    let diags = check("NOT VALID SQL *** ###");
    // Either 0 violations (parse error skipped) or some violations — must not panic
    let _ = diags;
}
