use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::undelimited_date_string::UndelimitedDateString;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    UndelimitedDateString.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UndelimitedDateString.name(), "Ambiguous/UndelimitedDateString");
}

#[test]
fn yyyymmdd_violation() {
    let diags = check("SELECT * FROM t WHERE date = '20230101'");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/UndelimitedDateString");
}

#[test]
fn iso_format_no_violation() {
    let diags = check("SELECT * FROM t WHERE date = '2023-01-01'");
    assert!(diags.is_empty());
}

#[test]
fn eight_digits_invalid_month_no_violation() {
    // Month 13 is invalid
    let diags = check("SELECT * FROM t WHERE date = '20231301'");
    assert!(diags.is_empty());
}

#[test]
fn eight_digits_invalid_day_no_violation() {
    // Day 32 is invalid
    let diags = check("SELECT * FROM t WHERE date = '20230132'");
    assert!(diags.is_empty());
}

#[test]
fn six_digits_no_violation() {
    // Not exactly 8 digits
    let diags = check("SELECT * FROM t WHERE val = '202301'");
    assert!(diags.is_empty());
}

#[test]
fn ten_digits_no_violation() {
    // More than 8 digits
    let diags = check("SELECT * FROM t WHERE val = '2023010101'");
    assert!(diags.is_empty());
}

#[test]
fn date_with_time_no_violation() {
    let diags = check("SELECT * FROM t WHERE ts = '2023-01-01 10:00:00'");
    assert!(diags.is_empty());
}

#[test]
fn yyyymmdd_in_comment_no_violation() {
    let diags = check("-- '20230101'\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn yyyymmdd_message_contains_value() {
    let diags = check("SELECT * FROM t WHERE date = '20230101'");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("20230101"),
        "expected message to contain '20230101', got: {}",
        diags[0].message
    );
}

#[test]
fn yyyymmdd_message_contains_suggested_fix() {
    let diags = check("SELECT * FROM t WHERE date = '20230101'");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("2023-01-01"),
        "expected message to contain '2023-01-01', got: {}",
        diags[0].message
    );
}

#[test]
fn multiple_violations() {
    let diags = check("SELECT * FROM t WHERE start_date = '20230101' AND end_date = '20231231'");
    assert_eq!(diags.len(), 2);
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}
