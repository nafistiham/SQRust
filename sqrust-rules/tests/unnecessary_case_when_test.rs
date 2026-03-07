use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::unnecessary_case_when::UnnecessaryCaseWhen;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    UnnecessaryCaseWhen.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnnecessaryCaseWhen.name(), "Convention/UnnecessaryCaseWhen");
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    let diags = UnnecessaryCaseWhen.check(&ctx);
    assert_eq!(diags.len(), 0);
}

#[test]
fn complex_case_no_violation() {
    let diags = check(
        "SELECT CASE WHEN a = 1 THEN 'x' WHEN a = 2 THEN 'y' ELSE 'z' END FROM t",
    );
    assert_eq!(diags.len(), 0);
}

#[test]
fn case_true_false_one_violation() {
    let diags = check(
        "SELECT CASE WHEN active = 1 THEN TRUE ELSE FALSE END FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_false_true_one_violation() {
    let diags = check(
        "SELECT CASE WHEN active = 1 THEN FALSE ELSE TRUE END FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_one_zero_one_violation() {
    let diags = check(
        "SELECT CASE WHEN active = 1 THEN 1 ELSE 0 END FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_zero_one_one_violation() {
    let diags = check(
        "SELECT CASE WHEN active = 1 THEN 0 ELSE 1 END FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_without_else_no_violation() {
    let diags = check("SELECT CASE WHEN a = 1 THEN TRUE END FROM t");
    assert_eq!(diags.len(), 0);
}

#[test]
fn case_multi_when_no_violation() {
    let diags = check(
        "SELECT CASE WHEN a = 1 THEN TRUE WHEN a = 2 THEN FALSE ELSE TRUE END FROM t",
    );
    assert_eq!(diags.len(), 0);
}

#[test]
fn case_returning_string_no_violation() {
    let diags = check(
        "SELECT CASE WHEN a = 1 THEN 'yes' ELSE 'no' END FROM t",
    );
    assert_eq!(diags.len(), 0);
}

#[test]
fn message_mentions_simplified() {
    let diags = check(
        "SELECT CASE WHEN active = 1 THEN TRUE ELSE FALSE END FROM t",
    );
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("simplified") || msg.contains("boolean"),
        "message should mention 'simplified' or 'boolean', got: {msg}"
    );
}

#[test]
fn line_nonzero() {
    let diags = check(
        "SELECT CASE WHEN active = 1 THEN TRUE ELSE FALSE END FROM t",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

#[test]
fn col_nonzero() {
    let diags = check(
        "SELECT CASE WHEN active = 1 THEN TRUE ELSE FALSE END FROM t",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn two_cases_both_violations() {
    let sql = concat!(
        "SELECT ",
        "CASE WHEN a = 1 THEN TRUE ELSE FALSE END, ",
        "CASE WHEN b = 2 THEN 1 ELSE 0 END ",
        "FROM t"
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}
