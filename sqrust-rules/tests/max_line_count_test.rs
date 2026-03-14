use sqrust_core::FileContext;
use sqrust_rules::layout::max_line_count::MaxLineCount;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    MaxLineCount::default().check(&ctx)
}

fn check_with_max(sql: &str, max_lines: usize) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    MaxLineCount { max_lines }.check(&ctx)
}

fn make_sql(n_lines: usize) -> String {
    "SELECT 1\n".repeat(n_lines)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(MaxLineCount::default().name(), "Layout/MaxLineCount");
}

#[test]
fn default_max_is_500() {
    assert_eq!(MaxLineCount::default().max_lines, 500);
}

#[test]
fn ten_line_file_has_no_violations() {
    let diags = check(&make_sql(10));
    assert!(diags.is_empty());
}

#[test]
fn file_at_limit_500_has_no_violations() {
    let diags = check(&make_sql(500));
    assert!(diags.is_empty());
}

#[test]
fn file_at_501_lines_has_one_violation() {
    let diags = check(&make_sql(501));
    assert_eq!(diags.len(), 1);
}

#[test]
fn custom_max_10_with_11_line_file_has_one_violation() {
    let diags = check_with_max(&make_sql(11), 10);
    assert_eq!(diags.len(), 1);
}

#[test]
fn custom_max_10_with_10_line_file_has_no_violations() {
    let diags = check_with_max(&make_sql(10), 10);
    assert!(diags.is_empty());
}

#[test]
fn custom_max_5_with_6_line_file_has_one_violation() {
    let diags = check_with_max(&make_sql(6), 5);
    assert_eq!(diags.len(), 1);
}

#[test]
fn empty_file_has_no_violations() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn single_line_has_no_violations() {
    let diags = check("SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn only_one_violation_per_file() {
    // A huge file still emits exactly one diagnostic.
    let diags = check(&make_sql(1000));
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_line_count_and_max() {
    let diags = check(&make_sql(501));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("501"),
        "message should contain actual line count"
    );
    assert!(
        diags[0].message.contains("500"),
        "message should contain max line count"
    );
}

#[test]
fn diagnostic_is_at_line_1_col_1() {
    let diags = check(&make_sql(501));
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn diagnostic_rule_name_is_correct() {
    let diags = check(&make_sql(501));
    assert_eq!(diags[0].rule, "Layout/MaxLineCount");
}
