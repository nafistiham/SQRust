use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::unnecessary_else_null::UnnecessaryElseNull;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    UnnecessaryElseNull.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnnecessaryElseNull.name(), "UnnecessaryElseNull");
}

#[test]
fn parse_error_returns_no_violations() {
    // Text-based rules do not depend on parse success, but parse errors should
    // still be handled gracefully (no panic). We just ensure it returns a Vec.
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    let diags = UnnecessaryElseNull.check(&ctx);
    // No ELSE NULL pattern in the garbage string, so 0 violations.
    assert_eq!(diags.len(), 0);
}

#[test]
fn simple_case_with_else_null_one_violation() {
    let diags = check("SELECT CASE WHEN x = 1 THEN 1 ELSE NULL END FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_with_else_value_no_violation() {
    let diags = check("SELECT CASE WHEN x = 1 THEN 1 ELSE 0 END FROM t");
    assert_eq!(diags.len(), 0);
}

#[test]
fn case_without_else_no_violation() {
    let diags = check("SELECT CASE WHEN x = 1 THEN 1 END FROM t");
    assert_eq!(diags.len(), 0);
}

#[test]
fn else_null_in_line_comment_no_violation() {
    let diags = check("SELECT 1 -- ELSE NULL is the default");
    assert_eq!(diags.len(), 0);
}

#[test]
fn else_null_inside_string_no_violation() {
    let diags = check("SELECT 'ELSE NULL' FROM t");
    assert_eq!(diags.len(), 0);
}

#[test]
fn else_null_inside_block_comment_no_violation() {
    let diags = check("SELECT 1 /* ELSE NULL is redundant */");
    assert_eq!(diags.len(), 0);
}

#[test]
fn nested_case_both_have_else_null_two_violations() {
    let sql = "SELECT CASE WHEN a = 1 THEN CASE WHEN b = 2 THEN 'x' ELSE NULL END ELSE NULL END FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn multiple_statements_one_violation() {
    let sql = "SELECT 1;\nSELECT CASE WHEN x = 1 THEN 1 ELSE NULL END FROM t;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_case_else_null_one_violation() {
    let diags = check("SELECT CASE WHEN x = 1 THEN 1 else null END FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_format_is_correct() {
    let diags = check("SELECT CASE WHEN x = 1 THEN 1 ELSE NULL END FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "ELSE NULL is redundant in CASE expression; omit ELSE to get the same result"
    );
}

#[test]
fn else_string_null_no_violation() {
    // 'NULL' is a string literal, not the NULL keyword
    let diags = check("SELECT CASE WHEN x = 1 THEN 1 ELSE 'NULL' END FROM t");
    assert_eq!(diags.len(), 0);
}

#[test]
fn line_points_to_else_keyword() {
    // ELSE NULL is on line 2
    let sql = "SELECT\n  CASE WHEN x = 1 THEN 1 ELSE NULL END\nFROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn col_points_to_else_keyword() {
    // "SELECT CASE WHEN x = 1 THEN 1 ELSE NULL END FROM t"
    //  1234567890123456789012345678901234
    // ELSE starts at col 32
    let sql = "SELECT CASE WHEN x = 1 THEN 1 ELSE NULL END FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 31);
}
