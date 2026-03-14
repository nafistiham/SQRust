use sqrust_core::FileContext;
use sqrust_rules::convention::string_agg_separator::StringAggSeparator;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    StringAggSeparator.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(StringAggSeparator.name(), "Convention/StringAggSeparator");
}

#[test]
fn group_concat_violation() {
    let diags = check("SELECT GROUP_CONCAT(col SEPARATOR ',') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn listagg_violation() {
    let diags = check("SELECT LISTAGG(col, ',') WITHIN GROUP (ORDER BY col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn group_concat_case_insensitive() {
    let diags = check("SELECT group_concat(col separator ',') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn listagg_case_insensitive() {
    let diags = check("SELECT listagg(col, ',') WITHIN GROUP (ORDER BY col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn string_agg_no_violation() {
    let diags = check("SELECT STRING_AGG(col, ',') FROM t");
    assert!(diags.is_empty());
}

#[test]
fn group_concat_in_select_violation() {
    let diags = check("SELECT id, GROUP_CONCAT(name SEPARATOR '; ') AS names FROM employees GROUP BY id");
    assert_eq!(diags.len(), 1);
}

#[test]
fn listagg_in_cte_violation() {
    let diags = check(
        "WITH cte AS (SELECT LISTAGG(val, ',') WITHIN GROUP (ORDER BY val) AS v FROM t) SELECT * FROM cte",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn group_concat_in_string_no_violation() {
    let diags = check("SELECT 'GROUP_CONCAT example' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn group_concat_in_comment_no_violation() {
    let diags = check("-- GROUP_CONCAT\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn multiple_group_concat_multiple_violations() {
    let diags = check(
        "SELECT GROUP_CONCAT(a SEPARATOR ','), GROUP_CONCAT(b SEPARATOR ';') FROM t",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn group_concat_message_content() {
    let diags = check("SELECT GROUP_CONCAT(col SEPARATOR ',') FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    assert!(
        msg.contains("STRING_AGG"),
        "message should mention STRING_AGG, got: {msg}"
    );
}

#[test]
fn listagg_message_content() {
    let diags = check("SELECT LISTAGG(col, ',') WITHIN GROUP (ORDER BY col) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    assert!(
        msg.contains("STRING_AGG"),
        "message should mention STRING_AGG, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT GROUP_CONCAT(col SEPARATOR ',') FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
