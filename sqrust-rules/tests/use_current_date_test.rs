use sqrust_core::FileContext;
use sqrust_rules::convention::use_current_date::UseCurrentDate;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    UseCurrentDate.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UseCurrentDate.name(), "Convention/UseCurrentDate");
}

// GETDATE tests

#[test]
fn getdate_basic_violation() {
    let diags = check("SELECT GETDATE() FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn getdate_lowercase_violation() {
    let diags = check("SELECT getdate() FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn getdate_message_is_correct() {
    let diags = check("SELECT GETDATE() FROM t");
    assert!(!diags.is_empty());
    assert_eq!(
        diags[0].message,
        "GETDATE() is SQL Server-specific; use CURRENT_TIMESTAMP for standard SQL"
    );
}

// GETUTCDATE tests

#[test]
fn getutcdate_basic_violation() {
    let diags = check("SELECT GETUTCDATE() FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn getutcdate_message_is_correct() {
    let diags = check("SELECT GETUTCDATE() FROM t");
    assert!(!diags.is_empty());
    assert_eq!(
        diags[0].message,
        "GETUTCDATE() is SQL Server-specific; use CURRENT_TIMESTAMP AT TIME ZONE 'UTC' for standard SQL"
    );
}

// SYSDATE tests

#[test]
fn sysdate_basic_violation() {
    // SYSDATE does not require parentheses in Oracle
    let diags = check("SELECT SYSDATE FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn sysdate_lowercase_violation() {
    let diags = check("SELECT sysdate FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn sysdate_message_is_correct() {
    let diags = check("SELECT SYSDATE FROM t");
    assert!(!diags.is_empty());
    assert_eq!(
        diags[0].message,
        "SYSDATE is Oracle-specific; use CURRENT_DATE or CURRENT_TIMESTAMP for standard SQL"
    );
}

// NOW tests

#[test]
fn now_basic_violation() {
    let diags = check("SELECT NOW() FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn now_message_is_correct() {
    let diags = check("SELECT NOW() FROM t");
    assert!(!diags.is_empty());
    assert_eq!(
        diags[0].message,
        "NOW() is MySQL/PostgreSQL-specific; use CURRENT_TIMESTAMP for standard SQL"
    );
}

// SYSDATETIME tests

#[test]
fn sysdatetime_basic_violation() {
    let diags = check("SELECT SYSDATETIME() FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn sysdatetime_message_is_correct() {
    let diags = check("SELECT SYSDATETIME() FROM t");
    assert!(!diags.is_empty());
    assert_eq!(
        diags[0].message,
        "SYSDATETIME() is SQL Server-specific; use CURRENT_TIMESTAMP for standard SQL"
    );
}

// SYSDATETIMEOFFSET tests

#[test]
fn sysdatetimeoffset_basic_violation() {
    let diags = check("SELECT SYSDATETIMEOFFSET() FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn sysdatetimeoffset_message_is_correct() {
    let diags = check("SELECT SYSDATETIMEOFFSET() FROM t");
    assert!(!diags.is_empty());
    assert_eq!(
        diags[0].message,
        "SYSDATETIMEOFFSET() is SQL Server-specific; use CURRENT_TIMESTAMP for standard SQL"
    );
}

// No-violation tests

#[test]
fn current_timestamp_no_violation() {
    let diags = check("SELECT CURRENT_TIMESTAMP FROM t");
    assert!(diags.is_empty());
}

#[test]
fn current_date_no_violation() {
    let diags = check("SELECT CURRENT_DATE FROM t");
    assert!(diags.is_empty());
}

#[test]
fn getdate_in_string_no_violation() {
    let diags = check("SELECT 'GETDATE()' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn now_in_comment_no_violation() {
    let diags = check("-- NOW() returns current time\nSELECT col FROM t");
    assert!(diags.is_empty());
}

// Word-boundary tests

#[test]
fn sysdate_as_column_prefix_no_violation() {
    // "sysdate_col" should not flag "sysdate" since there is a word char after
    let diags = check("SELECT sysdate_col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn now_as_column_suffix_no_violation() {
    // "right_now" — word char before "now"
    let diags = check("SELECT right_now FROM t");
    assert!(diags.is_empty());
}

// Line/col reporting

#[test]
fn line_col_is_nonzero() {
    let diags = check("SELECT GETDATE() FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
