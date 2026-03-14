use sqrust_core::FileContext;
use sqrust_rules::convention::no_rownum::NoRownum;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoRownum.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoRownum.name(), "Convention/NoRownum");
}

#[test]
fn rownum_basic_violation() {
    let diags = check("SELECT * FROM t WHERE ROWNUM <= 10");
    assert_eq!(diags.len(), 1);
}

#[test]
fn rownum_lowercase_violation() {
    let diags = check("SELECT * FROM t WHERE rownum <= 10");
    assert_eq!(diags.len(), 1);
}

#[test]
fn rownum_mixed_case_violation() {
    let diags = check("SELECT * FROM t WHERE RowNum <= 10");
    assert_eq!(diags.len(), 1);
}

#[test]
fn rownum_in_select_list_violation() {
    let diags = check("SELECT ROWNUM, col FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn rownum_in_order_by_violation() {
    let diags = check("SELECT * FROM t ORDER BY ROWNUM");
    assert_eq!(diags.len(), 1);
}

#[test]
fn rownum_in_string_no_violation() {
    let diags = check("SELECT 'ROWNUM' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn rownum_in_comment_no_violation() {
    let diags = check("-- use ROWNUM for pagination\nSELECT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_rownum_no_violation() {
    let diags = check("SELECT * FROM t WHERE id <= 10");
    assert!(diags.is_empty());
}

#[test]
fn row_number_function_no_violation() {
    // ROW_NUMBER() is the standard; should NOT be flagged (has underscore)
    let diags = check("SELECT ROW_NUMBER() OVER (ORDER BY col) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn rownum_identifier_prefix_no_violation() {
    // e.g. rownum_value should not be flagged (word boundary check)
    let diags = check("SELECT rownum_value FROM t");
    assert!(diags.is_empty());
}

#[test]
fn rownum_identifier_suffix_no_violation() {
    // e.g. my_rownum should not be flagged
    let diags = check("SELECT my_rownum FROM t");
    assert!(diags.is_empty());
}

#[test]
fn multiple_rownum_violations() {
    let diags = check("SELECT ROWNUM FROM t WHERE ROWNUM <= 5");
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_contains_rownum_and_alternative() {
    let diags = check("SELECT * FROM t WHERE ROWNUM <= 10");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("ROWNUM"),
        "message should contain 'ROWNUM', got: {msg}"
    );
    assert!(
        upper.contains("ROW_NUMBER") || upper.contains("FETCH FIRST"),
        "message should mention ROW_NUMBER or FETCH FIRST, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM t WHERE ROWNUM <= 10");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn rownum_in_cte_violation() {
    let diags = check("WITH c AS (SELECT * FROM t WHERE ROWNUM <= 100) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}
