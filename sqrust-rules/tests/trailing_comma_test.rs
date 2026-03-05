use sqrust_core::FileContext;
use sqrust_rules::convention::trailing_comma::TrailingComma;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    TrailingComma.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(TrailingComma.name(), "Convention/TrailingComma");
}

#[test]
fn trailing_comma_before_from_is_flagged() {
    let diags = check("SELECT col1, col2, FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Convention/TrailingComma");
}

#[test]
fn trailing_comma_message_is_correct() {
    let diags = check("SELECT col1, col2, FROM t");
    assert_eq!(diags[0].message, "Trailing comma before SQL keyword");
}

#[test]
fn no_trailing_comma_before_from_is_clean() {
    let diags = check("SELECT col1, col2 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn multiline_trailing_comma_before_from_is_flagged() {
    let diags = check("SELECT col1,\n  col2,\n  FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn trailing_comma_before_where_is_flagged() {
    let diags = check("SELECT col1, col2, WHERE col1 > 1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn trailing_comma_before_group_by_is_flagged() {
    let diags = check("SELECT col1, col2, GROUP BY col1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn trailing_comma_before_order_by_is_flagged() {
    let diags = check("SELECT col1, col2, ORDER BY col1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn trailing_comma_before_having_is_flagged() {
    let diags = check("SELECT col1, col2, HAVING COUNT(*) > 1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn trailing_comma_before_union_is_flagged() {
    let diags = check("SELECT col1, col2, UNION SELECT col1, col2");
    assert_eq!(diags.len(), 1);
}

#[test]
fn trailing_comma_in_comment_is_ignored() {
    let diags = check("-- SELECT col1, FROM t\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn trailing_comma_in_string_is_ignored() {
    let diags = check("SELECT 'col1,' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn fix_removes_trailing_comma() {
    let c = ctx("SELECT col1, col2, FROM t");
    let fixed = TrailingComma.fix(&c).expect("fix should produce output");
    assert_eq!(fixed, "SELECT col1, col2 FROM t");
}

#[test]
fn trailing_comma_before_limit_is_flagged() {
    let diags = check("SELECT col1, col2, LIMIT 10");
    assert_eq!(diags.len(), 1);
}

#[test]
fn trailing_comma_before_intersect_is_flagged() {
    let diags = check("SELECT col1, col2, INTERSECT SELECT col1, col2");
    assert_eq!(diags.len(), 1);
}

#[test]
fn trailing_comma_before_except_is_flagged() {
    let diags = check("SELECT col1, col2, EXCEPT SELECT col1, col2");
    assert_eq!(diags.len(), 1);
}
