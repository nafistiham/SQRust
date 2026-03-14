use sqrust_core::FileContext;
use sqrust_rules::convention::pivot_unpivot::PivotUnpivot;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    PivotUnpivot.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(PivotUnpivot.name(), "Convention/PivotUnpivot");
}

#[test]
fn pivot_keyword_one_violation() {
    let diags = check("SELECT * FROM t PIVOT (SUM(amount) FOR category IN ('A', 'B')) p");
    assert_eq!(diags.len(), 1);
}

#[test]
fn unpivot_keyword_one_violation() {
    let diags = check("SELECT * FROM t UNPIVOT (amount FOR category IN (col_a, col_b)) u");
    assert_eq!(diags.len(), 1);
}

#[test]
fn pivot_case_insensitive() {
    let diags = check("SELECT * FROM t pivot (SUM(v) FOR c IN ('x')) p");
    assert_eq!(diags.len(), 1);
}

#[test]
fn unpivot_case_insensitive() {
    let diags = check("SELECT * FROM t unpivot (v FOR c IN (col_a)) u");
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_pivot_no_violation() {
    let diags = check("SELECT a, b, c FROM t WHERE d = 1");
    assert!(diags.is_empty());
}

#[test]
fn pivot_in_column_name_no_violation() {
    let diags = check("SELECT pivot_data FROM t");
    assert!(diags.is_empty());
}

#[test]
fn unpivot_in_column_name_no_violation() {
    let diags = check("SELECT unpivot_result FROM t");
    assert!(diags.is_empty());
}

#[test]
fn both_pivot_and_unpivot_two_violations() {
    let diags = check(
        "SELECT * FROM t PIVOT (SUM(v) FOR c IN ('x')) p UNPIVOT (v2 FOR c2 IN (col1)) u",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn pivot_message_mentions_case_when() {
    let diags = check("SELECT * FROM t PIVOT (SUM(v) FOR c IN ('x')) p");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("CASE WHEN"),
        "message should mention CASE WHEN, got: {msg}"
    );
}

#[test]
fn unpivot_message_mentions_union_all() {
    let diags = check("SELECT * FROM t UNPIVOT (v FOR c IN (col_a)) u");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("UNION ALL"),
        "message should mention UNION ALL, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM t PIVOT (SUM(v) FOR c IN ('x')) p");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn pivot_in_string_no_violation() {
    let diags = check("SELECT 'use pivot here' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn pivot_in_comment_no_violation() {
    let diags = check("-- PIVOT example\nSELECT a FROM t");
    assert!(diags.is_empty());
}
