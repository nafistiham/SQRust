use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::case_when_same_result::CaseWhenSameResult;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CaseWhenSameResult.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CaseWhenSameResult.name(), "Ambiguous/CaseWhenSameResult");
}

#[test]
fn all_same_string_violation() {
    let diags = check("SELECT CASE WHEN a = 1 THEN 'yes' WHEN a = 2 THEN 'yes' ELSE 'yes' END");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/CaseWhenSameResult");
}

#[test]
fn different_results_no_violation() {
    let diags = check("SELECT CASE WHEN a = 1 THEN 'yes' ELSE 'no' END");
    assert!(diags.is_empty());
}

#[test]
fn all_same_integer_violation() {
    let diags = check("SELECT CASE WHEN x > 0 THEN 1 WHEN x < 0 THEN 1 ELSE 1 END");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/CaseWhenSameResult");
}

#[test]
fn null_null_violation() {
    let diags = check("SELECT CASE WHEN a = 1 THEN NULL ELSE NULL END");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/CaseWhenSameResult");
}

#[test]
fn two_whens_same_violation() {
    let diags = check("SELECT CASE WHEN a = 1 THEN 42 WHEN a = 2 THEN 42 END");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/CaseWhenSameResult");
}

#[test]
fn single_when_no_else_no_violation() {
    // Only one branch — not enough branches to conclude they're all the same.
    let diags = check("SELECT CASE WHEN a = 1 THEN 'yes' END");
    assert!(diags.is_empty());
}

#[test]
fn case_in_string_no_violation() {
    let diags = check("SELECT 'CASE WHEN a = 1 THEN 1 ELSE 1 END'");
    assert!(diags.is_empty());
}

#[test]
fn case_in_comment_no_violation() {
    let diags = check("-- CASE WHEN a = 1 THEN 1 ELSE 1 END\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn complex_then_expression_no_violation() {
    // THEN with a multi-token expression — not a simple literal, skip.
    let diags = check("SELECT CASE WHEN a = 1 THEN a + b ELSE 'x' END");
    assert!(diags.is_empty());
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn case_insensitive_string_values_violation() {
    // 'YES' and 'yes' should be treated as same value.
    let diags = check("SELECT CASE WHEN a = 1 THEN 'YES' ELSE 'yes' END");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/CaseWhenSameResult");
}

#[test]
fn nested_case_outer_violation() {
    // The outer CASE has all branches returning the same literal.
    let sql = "SELECT CASE WHEN x = 1 THEN 1 ELSE 1 END";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_branches_all_same_violation() {
    let diags = check(
        "SELECT CASE WHEN a = 1 THEN 7 WHEN a = 2 THEN 7 WHEN a = 3 THEN 7 ELSE 7 END",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/CaseWhenSameResult");
}
