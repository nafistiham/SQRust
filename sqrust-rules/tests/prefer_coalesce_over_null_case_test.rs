use sqrust_core::FileContext;
use sqrust_rules::convention::prefer_coalesce_over_null_case::PreferCoalesceOverNullCase;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    PreferCoalesceOverNullCase.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        PreferCoalesceOverNullCase.name(),
        "Convention/PreferCoalesceOverNullCase"
    );
}

#[test]
fn is_null_case_violation() {
    let diags = check("SELECT CASE WHEN col IS NULL THEN 0 ELSE col END FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn is_null_case_lowercase_violation() {
    let diags = check("select case when col is null then 0 else col end from t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn is_null_in_where_no_violation() {
    // IS NULL used in WHERE without any CASE WHEN — no violation
    let diags = check("SELECT * FROM t WHERE col IS NULL");
    assert!(diags.is_empty());
}

#[test]
fn normal_case_no_violation() {
    let diags = check("SELECT CASE WHEN col = 1 THEN 'a' ELSE 'b' END FROM t");
    assert!(diags.is_empty());
}

#[test]
fn is_null_case_in_string_no_violation() {
    let diags = check("SELECT 'CASE WHEN x IS NULL THEN 0 ELSE x END' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn is_null_case_in_comment_no_violation() {
    let diags = check("-- CASE WHEN x IS NULL THEN 0 ELSE x END\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn multiple_violations() {
    let sql = "SELECT CASE WHEN a IS NULL THEN 0 ELSE a END, \
               CASE WHEN b IS NULL THEN '' ELSE b END FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn is_not_null_no_violation() {
    // IS NOT NULL is a different pattern — should not be flagged
    let diags = check("SELECT CASE WHEN col IS NOT NULL THEN col ELSE 0 END FROM t");
    assert!(diags.is_empty());
}

#[test]
fn message_content() {
    let diags = check("SELECT CASE WHEN col IS NULL THEN 0 ELSE col END FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    assert!(
        msg.contains("COALESCE"),
        "message should contain 'COALESCE', got: {msg}"
    );
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn case_when_without_is_null_no_violation() {
    let diags = check("SELECT CASE WHEN col = 0 THEN 1 ELSE 0 END FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nested_case_violation() {
    // CASE WHEN IS NULL THEN ... ELSE ... inside a larger query
    let sql = "SELECT id, CASE WHEN name IS NULL THEN 'unknown' ELSE name END AS display_name FROM users";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
