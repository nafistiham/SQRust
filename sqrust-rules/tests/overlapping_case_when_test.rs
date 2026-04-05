use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::overlapping_case_when::OverlappingCaseWhen;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    OverlappingCaseWhen.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(OverlappingCaseWhen.name(), "Ambiguous/OverlappingCaseWhen");
}

#[test]
fn when_true_violation() {
    let diags = check("SELECT CASE WHEN TRUE THEN 1 ELSE 2 END");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/OverlappingCaseWhen");
}

#[test]
fn when_1_eq_1_violation() {
    let diags = check("SELECT CASE WHEN 1=1 THEN 1 ELSE 2 END");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/OverlappingCaseWhen");
}

#[test]
fn when_1_eq_1_spaced_violation() {
    let diags = check("SELECT CASE WHEN 1 = 1 THEN 1 ELSE 2 END");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/OverlappingCaseWhen");
}

#[test]
fn when_true_lowercase_violation() {
    let diags = check("SELECT CASE WHEN true THEN 1 END");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/OverlappingCaseWhen");
}

#[test]
fn when_normal_condition_no_violation() {
    let diags = check("SELECT CASE WHEN x=1 THEN 'a' END");
    assert!(diags.is_empty());
}

#[test]
fn when_true_in_string_no_violation() {
    let diags = check("SELECT 'CASE WHEN TRUE THEN 1 END'");
    assert!(diags.is_empty());
}

#[test]
fn when_true_in_comment_no_violation() {
    let diags = check("-- CASE WHEN TRUE THEN 1 END\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn when_false_no_violation() {
    // FALSE is always false — different issue, not flagged by this rule
    let diags = check("SELECT CASE WHEN FALSE THEN 1 ELSE 2 END");
    assert!(diags.is_empty());
}

#[test]
fn when_null_no_violation() {
    let diags = check("SELECT CASE WHEN NULL THEN 1 ELSE 2 END");
    assert!(diags.is_empty());
}

#[test]
fn when_1_eq_1_mixed_case() {
    let diags = check("SELECT case when 1=1 then 'x' end");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/OverlappingCaseWhen");
}

#[test]
fn multiple_violations() {
    let sql = "SELECT CASE WHEN TRUE THEN 1 END, CASE WHEN TRUE THEN 2 END";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}
