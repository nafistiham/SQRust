use sqrust_core::FileContext;
use sqrust_rules::convention::get_date::GetDate;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    GetDate.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(GetDate.name(), "Convention/GetDate");
}

#[test]
fn getdate_one_violation() {
    let diags = check("SELECT GETDATE() FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn getutcdate_one_violation() {
    let diags = check("SELECT GETUTCDATE() FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn now_no_violation() {
    let diags = check("SELECT NOW() FROM t");
    assert!(diags.is_empty());
}

#[test]
fn current_timestamp_no_violation() {
    let diags = check("SELECT CURRENT_TIMESTAMP FROM t");
    assert!(diags.is_empty());
}

#[test]
fn getdate_case_insensitive() {
    let diags = check("SELECT getdate() FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn getdate_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE created_at > GETDATE()");
    assert_eq!(diags.len(), 1);
}

#[test]
fn getdate_in_cte_violation() {
    let diags = check("WITH c AS (SELECT GETDATE() AS now FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn getdate_in_subquery_violation() {
    let diags = check("SELECT x FROM (SELECT GETDATE() AS now FROM t) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nested_getdate_violation() {
    let diags = check("SELECT COALESCE(GETDATE(), NULL) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn both_getdate_and_getutcdate_two_violations() {
    let diags = check("SELECT GETDATE(), GETUTCDATE() FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn getdate_message_content() {
    let diags = check("SELECT GETDATE() FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("GETDATE"),
        "message should mention GETDATE, got: {msg}"
    );
    assert!(
        upper.contains("CURRENT_TIMESTAMP"),
        "message should mention CURRENT_TIMESTAMP, got: {msg}"
    );
}

#[test]
fn getutcdate_message_content() {
    let diags = check("SELECT GETUTCDATE() FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("GETUTCDATE"),
        "message should mention GETUTCDATE, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT GETDATE() FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
