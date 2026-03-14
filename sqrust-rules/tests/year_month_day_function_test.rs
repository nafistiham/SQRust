use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::year_month_day_function::YearMonthDayFunction;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    YearMonthDayFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(YearMonthDayFunction.name(), "Ambiguous/YearMonthDayFunction");
}

#[test]
fn year_violation() {
    let diags = check("SELECT YEAR(created_at) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/YearMonthDayFunction");
}

#[test]
fn month_violation() {
    let diags = check("SELECT MONTH(created_at) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/YearMonthDayFunction");
}

#[test]
fn day_violation() {
    let diags = check("SELECT DAY(created_at) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/YearMonthDayFunction");
}

#[test]
fn hour_violation() {
    let diags = check("SELECT HOUR(created_at) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/YearMonthDayFunction");
}

#[test]
fn minute_violation() {
    let diags = check("SELECT MINUTE(created_at) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/YearMonthDayFunction");
}

#[test]
fn second_violation() {
    let diags = check("SELECT SECOND(created_at) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/YearMonthDayFunction");
}

#[test]
fn case_insensitive_year() {
    let diags = check("SELECT year(created_at) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_insensitive_month() {
    let diags = check("SELECT month(ts) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn extract_no_violation() {
    let diags = check("SELECT EXTRACT(YEAR FROM created_at) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn secondary_prefix_not_flagged() {
    // SECONDARY( should not be flagged - word boundary required
    let diags = check("SELECT SECONDARY(col) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn year_in_string_no_violation() {
    let diags = check("SELECT 'YEAR(col)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn year_in_line_comment_no_violation() {
    let diags = check("-- YEAR(col)\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn year_in_block_comment_no_violation() {
    let diags = check("/* YEAR(col) */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn multiple_functions_multiple_violations() {
    let sql = "SELECT YEAR(a), MONTH(b), DAY(c) FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 3);
}

#[test]
fn year_message_mentions_extract() {
    let diags = check("SELECT YEAR(col) FROM t");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_uppercase();
    assert!(
        msg.contains("EXTRACT"),
        "message should mention EXTRACT, got: {}",
        diags[0].message
    );
}

#[test]
fn month_message_mentions_extract() {
    let diags = check("SELECT MONTH(col) FROM t");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_uppercase();
    assert!(
        msg.contains("EXTRACT"),
        "message should mention EXTRACT, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT YEAR(col) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn two_year_calls_two_violations() {
    let diags = check("SELECT YEAR(a), YEAR(b) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn all_six_functions_six_violations() {
    let sql = "SELECT YEAR(a), MONTH(b), DAY(c), HOUR(d), MINUTE(e), SECOND(f) FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 6);
}

#[test]
fn day_message_mentions_extract() {
    let diags = check("SELECT DAY(col) FROM t");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_uppercase();
    assert!(
        msg.contains("EXTRACT"),
        "message should mention EXTRACT, got: {}",
        diags[0].message
    );
}
