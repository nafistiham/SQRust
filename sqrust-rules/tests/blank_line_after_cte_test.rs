use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::blank_line_after_cte::BlankLineAfterCte;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    BlankLineAfterCte.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(BlankLineAfterCte.name(), "Layout/BlankLineAfterCte");
}

#[test]
fn single_cte_no_violation() {
    assert!(check("WITH a AS (\n    SELECT 1\n)\nSELECT * FROM a").is_empty());
}

#[test]
fn no_cte_no_violation() {
    assert!(check("SELECT id FROM t WHERE id > 1").is_empty());
}

#[test]
fn two_ctes_with_blank_line_no_violation() {
    assert!(check("WITH a AS (\n    SELECT 1\n),\n\nb AS (\n    SELECT 2\n)\nSELECT * FROM a JOIN b ON 1=1").is_empty());
}

#[test]
fn two_ctes_no_blank_line_flagged() {
    let d = check("WITH a AS (\n    SELECT 1\n),\nb AS (\n    SELECT 2\n)\nSELECT * FROM a JOIN b ON 1=1");
    assert_eq!(d.len(), 1);
}

#[test]
fn three_ctes_two_missing_blank_lines_flagged() {
    let sql = "WITH a AS (\n    SELECT 1\n),\nb AS (\n    SELECT 2\n),\nc AS (\n    SELECT 3\n)\nSELECT * FROM a";
    let d = check(sql);
    assert_eq!(d.len(), 2);
}

#[test]
fn three_ctes_first_has_blank_second_missing_flagged() {
    let sql = "WITH a AS (\n    SELECT 1\n),\n\nb AS (\n    SELECT 2\n),\nc AS (\n    SELECT 3\n)\nSELECT * FROM a";
    let d = check(sql);
    assert_eq!(d.len(), 1);
}

#[test]
fn inline_ctes_flagged() {
    let d = check("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a JOIN b ON 1=1");
    assert_eq!(d.len(), 1);
}

#[test]
fn message_mentions_blank_line_or_cte() {
    let d = check("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("blank") || msg.contains("line") || msg.contains("cte"),
        "expected message to mention blank line or CTE, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Layout/BlankLineAfterCte");
}

#[test]
fn line_col_nonzero() {
    let d = check("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn cte_with_nested_parens_no_false_flag() {
    // CTE body has nested parens — should not confuse the depth tracker
    let sql = "WITH a AS (\n    SELECT COALESCE(x, 1) FROM t\n),\n\nb AS (\n    SELECT 2\n)\nSELECT * FROM a";
    assert!(check(sql).is_empty());
}

#[test]
fn single_inline_cte_no_violation() {
    assert!(check("WITH a AS (SELECT 1) SELECT * FROM a").is_empty());
}
