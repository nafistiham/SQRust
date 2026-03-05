use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::no_select_all::NoSelectAll;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoSelectAll.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoSelectAll.name(), "NoSelectAll");
}

#[test]
fn parse_error_returns_no_violations() {
    // Text-based rule. Garbage SQL with no SELECT ALL pattern → 0 violations.
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    let diags = NoSelectAll.check(&ctx);
    assert_eq!(diags.len(), 0);
}

#[test]
fn select_all_col_from_t_one_violation() {
    let diags = check("SELECT ALL col FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_col_from_t_no_violation() {
    let diags = check("SELECT col FROM t");
    assert_eq!(diags.len(), 0);
}

#[test]
fn select_distinct_no_violation() {
    let diags = check("SELECT DISTINCT col FROM t");
    assert_eq!(diags.len(), 0);
}

#[test]
fn select_all_in_line_comment_no_violation() {
    let diags = check("SELECT 1 -- SELECT ALL is redundant");
    assert_eq!(diags.len(), 0);
}

#[test]
fn select_all_in_block_comment_no_violation() {
    let diags = check("SELECT 1 /* SELECT ALL from t */");
    assert_eq!(diags.len(), 0);
}

#[test]
fn select_all_in_string_no_violation() {
    let diags = check("SELECT 'SELECT ALL' FROM t");
    assert_eq!(diags.len(), 0);
}

#[test]
fn multiple_select_all_multiple_violations() {
    let sql = "SELECT ALL a FROM t UNION ALL SELECT ALL b FROM u";
    let diags = check(sql);
    // "SELECT ALL a" → 1, "SELECT ALL b" → 1; the UNION ALL's "ALL" is not
    // preceded by SELECT so it must not be flagged.
    assert_eq!(diags.len(), 2);
}

#[test]
fn mixed_case_select_all_one_violation() {
    let diags = check("select all col from t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_allcols_no_violation() {
    // "ALL" is part of the identifier "ALLCOLS" — not a keyword boundary
    let diags = check("SELECT ALLCOLS FROM t");
    assert_eq!(diags.len(), 0);
}

#[test]
fn line_points_to_select_keyword() {
    let sql = "SELECT 1;\nSELECT ALL col FROM t;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn col_points_to_select_keyword() {
    // "SELECT ALL col FROM t"
    //  1234567
    // SELECT starts at col 1
    let diags = check("SELECT ALL col FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn message_format_is_correct() {
    let diags = check("SELECT ALL col FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "SELECT ALL is redundant; ALL is the default behavior, use SELECT without ALL"
    );
}
