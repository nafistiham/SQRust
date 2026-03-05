use sqrust_core::FileContext;
use sqrust_rules::convention::distinct_parenthesis::DistinctParenthesis;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DistinctParenthesis.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DistinctParenthesis.name(), "Convention/DistinctParenthesis");
}

#[test]
fn distinct_with_parens_is_flagged() {
    let diags = check("SELECT DISTINCT(col) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Convention/DistinctParenthesis");
}

#[test]
fn distinct_with_parens_message_is_correct() {
    let diags = check("SELECT DISTINCT(col) FROM t");
    assert_eq!(
        diags[0].message,
        "DISTINCT is not a function; write DISTINCT col instead of DISTINCT(col)"
    );
}

#[test]
fn distinct_with_parens_col_points_to_open_paren() {
    // "SELECT DISTINCT(col) FROM t"
    //  1234567890123456789
    // 'D' at 8, 'DISTINCT' is 8 chars, so '(' is at col 16
    let diags = check("SELECT DISTINCT(col) FROM t");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 16);
}

#[test]
fn distinct_with_multiple_cols_in_parens_is_flagged() {
    let diags = check("SELECT DISTINCT(col1, col2) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn distinct_without_parens_no_violation() {
    let diags = check("SELECT DISTINCT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn distinct_multiple_cols_without_parens_no_violation() {
    let diags = check("SELECT DISTINCT col, col2 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn count_distinct_no_violation() {
    // COUNT(DISTINCT col) — DISTINCT is inside parens, preceded by '('
    let diags = check("SELECT COUNT(DISTINCT col) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn lowercase_distinct_with_parens_is_flagged() {
    let diags = check("select distinct(col) from t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn distinct_in_line_comment_no_violation() {
    let diags = check("SELECT col -- DISTINCT(col)");
    assert!(diags.is_empty());
}

#[test]
fn distinct_in_block_comment_no_violation() {
    let diags = check("SELECT 1 /* DISTINCT(col) */");
    assert!(diags.is_empty());
}

#[test]
fn distinct_in_string_no_violation() {
    let diags = check("SELECT 'DISTINCT(col)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn multiple_violations_different_lines() {
    let sql = "SELECT DISTINCT(a)\nFROM t\nUNION ALL\nSELECT DISTINCT(b)\nFROM t2";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[1].line, 4);
}

#[test]
fn distinct_with_extra_spaces_before_paren_is_flagged() {
    // "SELECT DISTINCT  (col)" — whitespace between DISTINCT and '('
    let diags = check("SELECT DISTINCT  (col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_distinct_col_comma_other_no_violation() {
    let diags = check("SELECT DISTINCT(a), b FROM t");
    // DISTINCT(a) — 1 violation; the `, b` is separate
    assert_eq!(diags.len(), 1);
}

#[test]
fn distinct_followed_by_non_paren_no_violation() {
    // DISTINCT 1 — not a paren
    let diags = check("SELECT DISTINCT 1 FROM t");
    assert!(diags.is_empty());
}
