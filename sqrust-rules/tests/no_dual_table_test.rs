use sqrust_core::FileContext;
use sqrust_rules::convention::no_dual_table::NoDualTable;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoDualTable.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoDualTable.name(), "Convention/NoDualTable");
}

#[test]
fn from_dual_basic_violation() {
    let diags = check("SELECT 1 FROM DUAL");
    assert_eq!(diags.len(), 1);
}

#[test]
fn from_dual_lowercase_violation() {
    let diags = check("SELECT 1 FROM dual");
    assert_eq!(diags.len(), 1);
}

#[test]
fn from_dual_mixed_case_violation() {
    let diags = check("SELECT 1 FROM Dual");
    assert_eq!(diags.len(), 1);
}

#[test]
fn from_real_table_no_violation() {
    let diags = check("SELECT col FROM my_table");
    assert!(diags.is_empty());
}

#[test]
fn dual_as_column_no_violation() {
    // DUAL used as a column name (no FROM before it)
    let diags = check("SELECT dual FROM my_table");
    assert!(diags.is_empty());
}

#[test]
fn from_dual_in_string_no_violation() {
    let diags = check("SELECT 'FROM DUAL' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn from_dual_in_comment_no_violation() {
    let diags = check("-- SELECT 1 FROM DUAL\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn from_dual_in_subquery_violation() {
    let diags = check("SELECT a FROM (SELECT 1 FROM DUAL) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn from_dual_in_cte_violation() {
    let diags = check("WITH c AS (SELECT 1 FROM DUAL) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_from_dual_two_violations() {
    let diags = check("SELECT 1 FROM DUAL UNION ALL SELECT 2 FROM DUAL");
    assert_eq!(diags.len(), 2);
}

#[test]
fn from_dual_with_where_violation() {
    let diags = check("SELECT sysdate FROM DUAL WHERE 1=1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_dual_and_from() {
    let diags = check("SELECT 1 FROM DUAL");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("DUAL"),
        "message should contain 'DUAL', got: {msg}"
    );
    assert!(
        upper.contains("FROM"),
        "message should mention FROM, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT 1 FROM DUAL");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn from_dual_extra_whitespace_violation() {
    // Multiple spaces between FROM and DUAL
    let diags = check("SELECT 1 FROM  DUAL");
    assert_eq!(diags.len(), 1);
}

#[test]
fn from_dual_table_name_no_violation() {
    // DUAL as a prefix in a longer table name should not be flagged
    let diags = check("SELECT col FROM dual_results");
    assert!(diags.is_empty());
}

#[test]
fn from_dual_newline_violation() {
    let diags = check("SELECT 1\nFROM\nDUAL");
    assert_eq!(diags.len(), 1);
}
