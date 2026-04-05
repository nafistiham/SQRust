use sqrust_core::{FileContext, Rule};
use sqrust_rules::lint::call_statement::CallStatement;

fn check(sql: &str) -> Vec<String> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CallStatement
        .check(&ctx)
        .into_iter()
        .map(|d| format!("{}:{} {}", d.line, d.col, d.message))
        .collect()
}

fn violation_count(sql: &str) -> usize {
    let ctx = FileContext::from_source(sql, "test.sql");
    CallStatement.check(&ctx).len()
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CallStatement.name(), "Lint/CallStatement");
}

#[test]
fn call_simple_violation() {
    let results = check("CALL my_procedure()");
    assert_eq!(results.len(), 1);
    assert!(results[0].contains("CALL statement invokes a stored procedure"));
}

#[test]
fn call_with_args_violation() {
    let results = check("CALL update_stats(2023, 'Q1')");
    assert_eq!(results.len(), 1);
}

#[test]
fn call_lowercase_violation() {
    let results = check("call my_procedure()");
    assert_eq!(results.len(), 1);
}

#[test]
fn call_in_string_no_violation() {
    let results = check("SELECT 'CALL proc()' FROM t");
    assert_eq!(results.len(), 0);
}

#[test]
fn call_in_comment_no_violation() {
    let results = check("-- CALL proc()\nSELECT 1");
    assert_eq!(results.len(), 0);
}

#[test]
fn no_call_no_violation() {
    let results = check("SELECT * FROM t");
    assert_eq!(results.len(), 0);
}

#[test]
fn select_callback_no_violation() {
    // "callback" contains "call" but is not a standalone keyword
    let results = check("SELECT callback FROM t");
    assert_eq!(results.len(), 0);
}

#[test]
fn call_uppercase_violation() {
    let results = check("CALL my_proc()");
    assert_eq!(results.len(), 1);
}

#[test]
fn multiple_call_statements_two_violations() {
    let sql = "CALL proc_a();\nCALL proc_b();";
    assert_eq!(violation_count(sql), 2);
}

#[test]
fn call_with_schema_prefix_violation() {
    let results = check("CALL schema.procedure()");
    assert_eq!(results.len(), 1);
}

#[test]
fn call_mixed_case_violation() {
    let results = check("Call my_proc()");
    assert_eq!(results.len(), 1);
}

#[test]
fn empty_file_no_violation() {
    let results = check("");
    assert_eq!(results.len(), 0);
}
