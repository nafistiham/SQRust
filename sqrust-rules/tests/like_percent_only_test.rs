use sqrust_core::FileContext;
use sqrust_rules::convention::like_percent_only::LikePercentOnly;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    LikePercentOnly.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(LikePercentOnly.name(), "Convention/LikePercentOnly");
}

#[test]
fn parse_error_produces_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn like_single_percent_is_flagged() {
    let diags = check("SELECT * FROM t WHERE col LIKE '%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn like_percent_with_value_before_is_clean() {
    let diags = check("SELECT * FROM t WHERE col LIKE '%value%'");
    assert!(diags.is_empty());
}

#[test]
fn like_value_then_percent_is_clean() {
    let diags = check("SELECT * FROM t WHERE col LIKE '%value'");
    assert!(diags.is_empty());
}

#[test]
fn like_percent_then_value_is_clean() {
    let diags = check("SELECT * FROM t WHERE col LIKE 'value%'");
    assert!(diags.is_empty());
}

#[test]
fn not_like_single_percent_is_flagged() {
    let diags = check("SELECT * FROM t WHERE col NOT LIKE '%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn like_percent_in_line_comment_is_ignored() {
    let diags = check("-- WHERE col LIKE '%'\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn like_percent_in_string_is_ignored() {
    let diags = check("SELECT * FROM t WHERE col = 'LIKE '''%'''");
    assert!(diags.is_empty());
}

#[test]
fn like_percent_in_block_comment_is_ignored() {
    let diags = check("/* WHERE col LIKE '%' */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn like_double_percent_is_clean() {
    let diags = check("SELECT * FROM t WHERE col LIKE '%%'");
    assert!(diags.is_empty());
}

#[test]
fn lowercase_like_is_flagged() {
    let diags = check("SELECT * FROM t WHERE col like '%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn line_and_col_point_to_like_keyword() {
    let diags = check("SELECT * FROM t WHERE col LIKE '%'");
    assert_eq!(diags[0].line, 1);
    // "SELECT * FROM t WHERE col LIKE '%'"
    //  col 27 is where LIKE starts
    assert_eq!(diags[0].col, 27);
}

#[test]
fn like_single_percent_message_is_correct() {
    let diags = check("SELECT * FROM t WHERE col LIKE '%'");
    assert_eq!(
        diags[0].message,
        "LIKE '%' matches everything; use IS NOT NULL instead"
    );
}

#[test]
fn not_like_single_percent_message_is_correct() {
    let diags = check("SELECT * FROM t WHERE col NOT LIKE '%'");
    assert_eq!(
        diags[0].message,
        "NOT LIKE '%' matches nothing; use IS NULL instead"
    );
}

#[test]
fn not_like_col_points_to_like_keyword() {
    let diags = check("SELECT * FROM t WHERE col NOT LIKE '%'");
    // "SELECT * FROM t WHERE col NOT LIKE '%'"
    //  LIKE starts at col 31
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 31);
}
