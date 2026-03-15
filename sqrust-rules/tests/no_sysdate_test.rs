use sqrust_core::FileContext;
use sqrust_rules::convention::no_sysdate::NoSysdate;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoSysdate.check(&ctx)
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoSysdate.name(), "Convention/NoSysdate");
}

// ── No violation ──────────────────────────────────────────────────────────────

#[test]
fn current_date_no_violation() {
    let diags = check("SELECT CURRENT_DATE FROM t");
    assert!(diags.is_empty());
}

#[test]
fn current_timestamp_no_violation() {
    let diags = check("SELECT CURRENT_TIMESTAMP FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_sysdate_at_all_no_violation() {
    let diags = check("SELECT id, name FROM t WHERE id = 1");
    assert!(diags.is_empty());
}

#[test]
fn sysdate_in_string_no_violation() {
    let diags = check("SELECT 'SYSDATE' AS msg FROM t");
    assert!(diags.is_empty());
}

#[test]
fn sysdate_in_line_comment_no_violation() {
    let diags = check("-- SYSDATE\nSELECT CURRENT_DATE FROM t");
    assert!(diags.is_empty());
}

#[test]
fn sysdate_in_block_comment_no_violation() {
    let diags = check("/* use SYSDATE here */\nSELECT CURRENT_DATE FROM t");
    assert!(diags.is_empty());
}

#[test]
fn sysdate_as_column_prefix_no_violation() {
    // sysdate_col is a column name, not a standalone SYSDATE
    let diags = check("SELECT sysdate_col FROM t");
    assert!(diags.is_empty());
}

// ── Violations ────────────────────────────────────────────────────────────────

#[test]
fn sysdate_basic_violation() {
    let diags = check("SELECT SYSDATE FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn sysdate_lowercase_violation() {
    let diags = check("SELECT sysdate FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn sysdate_mixed_case_violation() {
    let diags = check("SELECT Sysdate FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn sysdate_in_where_clause_violation() {
    let diags = check("SELECT id FROM t WHERE created_at > SYSDATE");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_sysdate_multiple_violations() {
    let diags = check("SELECT SYSDATE, SYSDATE FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn sysdate_in_subquery_violation() {
    let diags = check("SELECT a FROM (SELECT SYSDATE AS d FROM t) sub");
    assert_eq!(diags.len(), 1);
}

// ── Message ───────────────────────────────────────────────────────────────────

#[test]
fn message_mentions_sysdate_and_current_date() {
    let diags = check("SELECT SYSDATE FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("SYSDATE"),
        "message should mention SYSDATE, got: {msg}"
    );
    assert!(
        upper.contains("CURRENT_DATE") || upper.contains("CURRENT_TIMESTAMP"),
        "message should suggest CURRENT_DATE or CURRENT_TIMESTAMP, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT SYSDATE FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn empty_source_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}
