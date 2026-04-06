use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::layout::group_by_column_per_line::GroupByColumnPerLine;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    GroupByColumnPerLine.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(GroupByColumnPerLine.name(), "Layout/GroupByColumnPerLine");
}

#[test]
fn two_columns_same_line_violation() {
    let sql = "SELECT a, b, COUNT(*) FROM t\nGROUP BY a, b";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Layout/GroupByColumnPerLine");
    assert_eq!(
        diags[0].message,
        "In multi-line GROUP BY, each column should be on its own line"
    );
}

#[test]
fn each_column_own_line_no_violation() {
    let sql = "SELECT a, b FROM t\nGROUP BY\n  a,\n  b";
    assert!(check(sql).is_empty());
}

#[test]
fn single_column_group_by_no_violation() {
    let sql = "SELECT a, COUNT(*) FROM t\nGROUP BY a";
    assert!(check(sql).is_empty());
}

#[test]
fn single_line_query_no_violation() {
    let sql = "SELECT a, b, COUNT(*) FROM t GROUP BY a, b";
    assert!(check(sql).is_empty());
}

#[test]
fn three_columns_violation() {
    let sql = "SELECT a, b, c, COUNT(*) FROM t\nGROUP BY a, b, c";
    let diags = check(sql);
    // Two commas with content after them on the same line
    assert!(!diags.is_empty());
}

#[test]
fn no_group_by_no_violation() {
    let sql = "SELECT a FROM t\nWHERE a = 1";
    assert!(check(sql).is_empty());
}

#[test]
fn group_by_in_comment_no_violation() {
    let sql = "-- GROUP BY a, b\nSELECT 1";
    assert!(check(sql).is_empty());
}

#[test]
fn comma_at_end_of_line_no_violation() {
    let sql = "SELECT a, b FROM t\nGROUP BY\n  a,\n  b";
    assert!(check(sql).is_empty());
}

#[test]
fn group_by_with_rollup_violation() {
    // "a, b" appear on the same line in a multi-line GROUP BY
    let sql = "SELECT a, b FROM t\nGROUP BY a, b WITH ROLLUP";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn group_by_one_and_count_no_violation() {
    // Only one real grouping column; COUNT(*) is an aggregate, not a GROUP BY column
    let sql = "SELECT a, COUNT(*) FROM t\nGROUP BY a";
    assert!(check(sql).is_empty());
}

#[test]
fn multiline_group_by_all_separate_no_violation() {
    let sql = "SELECT a, b, c, COUNT(*) FROM t\nGROUP BY\n  a,\n  b,\n  c";
    assert!(check(sql).is_empty());
}

#[test]
fn empty_file_no_violation() {
    assert!(check("").is_empty());
}
