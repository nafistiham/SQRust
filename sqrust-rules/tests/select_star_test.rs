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
    // "SELECT * FROM t"
    //  1234567 8
    // '*' is at col 8
    let diags = check("SELECT * FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 8);
}

#[test]
fn qualified_wildcard_flagged() {
    // "SELECT t.* FROM t"
    //  123456789 10
    // '*' is at col 10
    let diags = check("SELECT t.* FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 10);
}

#[test]
fn explicit_columns_no_violation() {
    let diags = check("SELECT a, b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn count_star_no_violation() {
    // COUNT(*) — '*' is preceded by '(' so must not be flagged
    let diags = check("SELECT COUNT(*) FROM t");
    assert!(diags.is_empty());
}

// Known limitation: arithmetic `a * b` may or may not be flagged depending
// on context. With this text-scan approach, `a * b` has '*' preceded by
// space and followed by space, which matches the standalone-star heuristic.
// This is a known false positive; the test documents actual behavior.
#[test]
fn arithmetic_star_behavior() {
    // "SELECT a * b FROM t" — '*' at col 10, preceded by space, followed by space.
    // Implementation will flag this as a false positive (known limitation).
    let diags = check("SELECT a * b FROM t");
    // Document actual behavior: the heuristic does flag this.
    assert_eq!(diags.len(), 1);
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
    // "SELECT *, a FROM t" — standalone '*' followed by ','
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
fn correct_col_number_for_select_star() {
    // "SELECT * FROM t" — '*' at col 8
    let diags = check("SELECT * FROM t");
    assert_eq!(diags[0].col, 8);
}

#[test]
fn select_star_violation_has_correct_message() {
    let diags = check("SELECT * FROM t");
    assert_eq!(diags[0].message, "Avoid SELECT *; list columns explicitly");
}
