use sqrust_core::FileContext;
use sqrust_rules::convention::count_star::CountStar;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CountStar.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CountStar.name(), "Convention/CountStar");
}

#[test]
fn count_one_flagged() {
    let diags = check("SELECT COUNT(1) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn count_star_no_violation() {
    let diags = check("SELECT COUNT(*) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn lowercase_count_one_flagged() {
    let diags = check("SELECT count(1) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_case_count_one_flagged() {
    let diags = check("SELECT Count(1) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn count_two_no_violation() {
    // COUNT(2) should not be flagged — only COUNT(1)
    let diags = check("SELECT COUNT(2) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn count_one_inside_string_no_violation() {
    let diags = check("SELECT 'COUNT(1)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn count_one_inside_line_comment_no_violation() {
    let diags = check("SELECT 1 -- COUNT(1)");
    assert!(diags.is_empty());
}

#[test]
fn count_one_inside_block_comment_no_violation() {
    let diags = check("SELECT 1 /* COUNT(1) */");
    assert!(diags.is_empty());
}

#[test]
fn count_one_on_second_line_correct_line_and_col() {
    // Line 1: "SELECT\n"
    // Line 2: "COUNT(1) FROM t"
    // 'C' of COUNT is at line 2, col 1
    let sql = "SELECT\nCOUNT(1) FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn discount_one_no_violation() {
    // "DISCOUNT(1)" — word char 'D' precedes 'C', so word boundary fails
    let diags = check("SELECT DISCOUNT(1) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn count_one_violation_has_correct_message() {
    let diags = check("SELECT COUNT(1) FROM t");
    assert_eq!(diags[0].message, "Use COUNT(*) instead of COUNT(1)");
}

#[test]
fn count_one_violation_col_is_c_position() {
    // "SELECT COUNT(1) FROM t"
    //  1234567 8
    // 'C' in COUNT is at col 8
    let diags = check("SELECT COUNT(1) FROM t");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 8);
}
