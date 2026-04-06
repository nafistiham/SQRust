use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::layout::order_by_column_per_line::OrderByColumnPerLine;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    OrderByColumnPerLine.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(OrderByColumnPerLine.name(), "Layout/OrderByColumnPerLine");
}

#[test]
fn two_columns_same_line_violation() {
    let sql = "SELECT a, b FROM t\nORDER BY a, b";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Layout/OrderByColumnPerLine");
    assert_eq!(
        diags[0].message,
        "In multi-line ORDER BY, each column should be on its own line"
    );
}

#[test]
fn each_column_own_line_no_violation() {
    let sql = "SELECT a, b FROM t\nORDER BY\n  a,\n  b";
    assert!(check(sql).is_empty());
}

#[test]
fn single_column_order_by_no_violation() {
    let sql = "SELECT a FROM t\nORDER BY a";
    assert!(check(sql).is_empty());
}

#[test]
fn single_line_query_no_violation() {
    let sql = "SELECT a, b FROM t ORDER BY a, b";
    assert!(check(sql).is_empty());
}

#[test]
fn three_columns_two_on_same_line_violation() {
    // "a, b" on same line — that comma has content after it on the same line
    let sql = "SELECT a,b,c FROM t\nORDER BY a, b,\n  c";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_order_by_no_violation() {
    let sql = "SELECT a FROM t\nWHERE a = 1";
    assert!(check(sql).is_empty());
}

#[test]
fn order_by_in_string_no_violation() {
    let sql = "SELECT 'no order by here' AS x FROM t\nWHERE a = 1";
    assert!(check(sql).is_empty());
}

#[test]
fn order_by_in_comment_no_violation() {
    // ORDER BY only appears in the comment; no real ORDER BY clause
    let sql = "-- ORDER BY a, b\nSELECT 1";
    assert!(check(sql).is_empty());
}

#[test]
fn comma_at_end_of_line_no_violation() {
    // Trailing comma style: comma at end of line, column continues on next line
    let sql = "SELECT a,b FROM t\nORDER BY\n  a,\n  b";
    assert!(check(sql).is_empty());
}

#[test]
fn order_by_desc_same_line_violation() {
    let sql = "SELECT a FROM t\nORDER BY a DESC, b ASC";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn single_column_with_desc_no_violation() {
    let sql = "SELECT a FROM t\nORDER BY a DESC";
    assert!(check(sql).is_empty());
}

#[test]
fn empty_file_no_violation() {
    assert!(check("").is_empty());
}
