use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::add_months_function::AddMonthsFunction;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    AddMonthsFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(AddMonthsFunction.name(), "Ambiguous/AddMonthsFunction");
}

#[test]
fn add_months_uppercase_violation() {
    let diags = check("SELECT ADD_MONTHS(hire_date, 3) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/AddMonthsFunction");
}

#[test]
fn add_months_lowercase_violation() {
    let diags = check("SELECT add_months(hire_date, 3) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn add_months_mixedcase_violation() {
    let diags = check("SELECT Add_Months(hire_date, 3) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn months_between_uppercase_violation() {
    let diags = check("SELECT MONTHS_BETWEEN(end_date, start_date) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/AddMonthsFunction");
}

#[test]
fn months_between_lowercase_violation() {
    let diags = check("SELECT months_between(end_date, start_date) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn add_months_message_mentions_interval() {
    let diags = check("SELECT ADD_MONTHS(col, 2) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("INTERVAL"),
        "message should mention INTERVAL, got: {}",
        diags[0].message
    );
}

#[test]
fn months_between_message_mentions_datediff() {
    let diags = check("SELECT MONTHS_BETWEEN(col1, col2) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_uppercase().contains("DATEDIFF"),
        "message should mention DATEDIFF, got: {}",
        diags[0].message
    );
}

#[test]
fn add_months_message_mentions_oracle() {
    let diags = check("SELECT ADD_MONTHS(col, 2) FROM t");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("oracle"),
        "message should mention Oracle, got: {}",
        diags[0].message
    );
}

#[test]
fn months_between_message_mentions_oracle() {
    let diags = check("SELECT MONTHS_BETWEEN(col1, col2) FROM t");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("oracle"),
        "message should mention Oracle, got: {}",
        diags[0].message
    );
}

#[test]
fn add_months_in_string_no_violation() {
    let diags = check("SELECT 'ADD_MONTHS(col, 2)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn add_months_in_line_comment_no_violation() {
    let diags = check("-- ADD_MONTHS(col, 2)\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn add_months_in_block_comment_no_violation() {
    let diags = check("/* ADD_MONTHS(col, 2) */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn months_between_in_string_no_violation() {
    let diags = check("SELECT 'MONTHS_BETWEEN(col1, col2)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn both_functions_two_violations() {
    let sql = "SELECT ADD_MONTHS(a, 3), MONTHS_BETWEEN(b, c) FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn word_boundary_add_months_not_flagged() {
    // XADD_MONTHS should not be flagged
    let diags = check("SELECT XADD_MONTHS(col, 2) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT ADD_MONTHS(col, 2) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn empty_source_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn standard_interval_no_violation() {
    let diags = check("SELECT col + INTERVAL 3 MONTH FROM t");
    assert!(diags.is_empty());
}
