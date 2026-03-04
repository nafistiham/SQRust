use sqrust_core::FileContext;
use sqrust_rules::convention::coalesce::Coalesce;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    Coalesce.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(Coalesce.name(), "Convention/Coalesce");
}

#[test]
fn isnull_flagged() {
    let diags = check("SELECT ISNULL(a, b) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nvl_flagged() {
    let diags = check("SELECT NVL(a, b) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nvl2_flagged() {
    let diags = check("SELECT NVL2(a, b, c) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn ifnull_flagged() {
    let diags = check("SELECT IFNULL(a, b) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_isnull_flagged() {
    let diags = check("SELECT isnull(a, b) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_nvl_flagged() {
    let diags = check("SELECT nvl(a, b) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn coalesce_no_violation() {
    let diags = check("SELECT COALESCE(a, b) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn isnull_without_paren_not_flagged() {
    // Not a function call — no '(' immediately following
    let diags = check("-- ISNULL");
    assert!(diags.is_empty());
}

#[test]
fn isnull_inside_single_quoted_string_skipped() {
    let diags = check("SELECT 'ISNULL(' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn isnull_inside_line_comment_skipped() {
    let diags = check("SELECT 1 -- ISNULL(a, b)");
    assert!(diags.is_empty());
}

#[test]
fn isnull_inside_block_comment_skipped() {
    let diags = check("SELECT 1 /* ISNULL(a, b) */");
    assert!(diags.is_empty());
}

#[test]
fn isnull_violation_has_correct_line_and_col() {
    // "SELECT ISNULL(a, b) FROM t"
    //  1234567
    // 'I' in ISNULL is at col 8
    let diags = check("SELECT ISNULL(a, b) FROM t");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 8);
}

#[test]
fn isnull_violation_has_correct_message() {
    let diags = check("SELECT ISNULL(a, b) FROM t");
    assert_eq!(diags[0].message, "Use COALESCE instead of ISNULL()");
}

#[test]
fn nvl_violation_message_uppercase() {
    let diags = check("SELECT nvl(a, b) FROM t");
    assert_eq!(diags[0].message, "Use COALESCE instead of NVL()");
}

#[test]
fn multiple_non_ansi_functions_all_flagged() {
    let sql = "SELECT ISNULL(a, 0), NVL(b, '') FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn function_on_second_line_correct_line_number() {
    let sql = "SELECT 1\nFROM t WHERE x = ISNULL(col, 0)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn isnull_name_without_paren_space_not_flagged() {
    // ISNULL followed by a space then '(' — NOT a match (requires immediate '(')
    let diags = check("SELECT ISNULL (a, b) FROM t");
    assert!(diags.is_empty());
}
