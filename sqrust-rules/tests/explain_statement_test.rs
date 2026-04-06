use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::explain_statement::ExplainStatement;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ExplainStatement.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ExplainStatement.name(), "Lint/ExplainStatement");
}

#[test]
fn explain_simple_violation() {
    let sql = "EXPLAIN SELECT * FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn explain_analyze_violation() {
    let sql = "EXPLAIN ANALYZE SELECT * FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn explain_query_plan_violation() {
    let sql = "EXPLAIN QUERY PLAN SELECT * FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn explain_lowercase_violation() {
    let sql = "explain select * from t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn explain_mixed_case_violation() {
    let sql = "Explain SELECT * FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn explain_in_string_no_violation() {
    let sql = "SELECT 'EXPLAIN SELECT 1' FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn explain_in_comment_no_violation() {
    let sql = "-- EXPLAIN SELECT * FROM t\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn no_explain_no_violation() {
    let sql = "SELECT * FROM t WHERE a = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn explanation_word_boundary_no_violation() {
    let sql = "SELECT explanation FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiple_explains_two_violations() {
    let sql = "EXPLAIN SELECT 1;\nEXPLAIN SELECT 2;";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn explain_uppercase_violation() {
    let sql = "EXPLAIN SELECT id FROM users";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("EXPLAIN"));
}

#[test]
fn empty_file_no_violation() {
    let sql = "";
    let diags = check(sql);
    assert!(diags.is_empty());
}
