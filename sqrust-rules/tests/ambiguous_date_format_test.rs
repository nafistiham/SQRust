use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::ambiguous_date_format::AmbiguousDateFormat;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    AmbiguousDateFormat.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(AmbiguousDateFormat.name(), "Ambiguous/AmbiguousDateFormat");
}

#[test]
fn iso_date_no_violation() {
    assert!(check("SELECT id FROM t WHERE dt = '2023-12-01'").is_empty());
}

#[test]
fn non_date_string_no_violation() {
    assert!(check("SELECT id FROM t WHERE name = 'hello'").is_empty());
}

#[test]
fn slash_date_mm_dd_yyyy_flagged() {
    let d = check("SELECT id FROM t WHERE dt = '12/01/2023'");
    assert_eq!(d.len(), 1);
}

#[test]
fn slash_date_d_m_yy_flagged() {
    let d = check("SELECT id FROM t WHERE dt = '1/5/24'");
    assert_eq!(d.len(), 1);
}

#[test]
fn slash_date_dd_mm_yyyy_flagged() {
    let d = check("SELECT id FROM t WHERE dt >= '01/12/2023'");
    assert_eq!(d.len(), 1);
}

#[test]
fn year_first_slash_no_violation() {
    // '2023/12/01' — year is first (>31), unambiguous
    assert!(check("SELECT id FROM t WHERE dt = '2023/12/01'").is_empty());
}

#[test]
fn two_slash_dates_flagged() {
    let d = check("SELECT id FROM t WHERE dt BETWEEN '01/01/2023' AND '12/31/2023'");
    assert_eq!(d.len(), 2);
}

#[test]
fn slash_date_in_comment_not_flagged() {
    assert!(check("SELECT id FROM t -- where dt = '12/01/2023'\nWHERE id > 1").is_empty());
}

#[test]
fn message_mentions_iso_or_format() {
    let d = check("SELECT id FROM t WHERE dt = '12/01/2023'");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("iso") || msg.contains("format") || msg.contains("locale") || msg.contains("ambiguous"),
        "expected message to mention ISO/format/locale, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM t WHERE dt = '12/01/2023'");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Ambiguous/AmbiguousDateFormat");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM t WHERE dt = '12/01/2023'");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn url_like_string_no_violation() {
    // Not a date pattern (three slashes, not two)
    assert!(check("SELECT id FROM t WHERE url = 'http://example.com/path'").is_empty());
}
