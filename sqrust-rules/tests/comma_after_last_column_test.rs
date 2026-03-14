use sqrust_core::FileContext;
use sqrust_rules::layout::comma_after_last_column::CommaAfterLastColumn;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(CommaAfterLastColumn.name(), "Layout/CommaAfterLastColumn");
}

// ── Basic violations ──────────────────────────────────────────────────────────

#[test]
fn trailing_comma_before_from_violation() {
    let diags = CommaAfterLastColumn.check(&ctx("SELECT a, b, FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_trailing_comma_no_violation() {
    let diags = CommaAfterLastColumn.check(&ctx("SELECT a, b FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn trailing_comma_before_from_multiline_violation() {
    let src = "SELECT\n    a,\n    b,\nFROM t";
    let diags = CommaAfterLastColumn.check(&ctx(src));
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_a_comma_from_multiline_violation() {
    let src = "SELECT a,\nFROM t";
    let diags = CommaAfterLastColumn.check(&ctx(src));
    assert_eq!(diags.len(), 1);
}

// ── No false positives for commas not before FROM ────────────────────────────

#[test]
fn comma_in_where_no_violation() {
    let diags = CommaAfterLastColumn.check(&ctx("SELECT a FROM t WHERE b IN (1, 2, 3)"));
    assert!(diags.is_empty());
}

#[test]
fn comma_in_function_no_violation() {
    let diags = CommaAfterLastColumn.check(&ctx("SELECT COALESCE(a, b) FROM t"));
    assert!(diags.is_empty());
}

// ── String and comment skipping ───────────────────────────────────────────────

#[test]
fn comma_then_from_in_string_no_violation() {
    let diags = CommaAfterLastColumn.check(&ctx("SELECT ', FROM t' FROM r"));
    assert!(diags.is_empty());
}

#[test]
fn comma_then_from_in_comment_no_violation() {
    let src = "-- , FROM t\nSELECT a FROM t";
    let diags = CommaAfterLastColumn.check(&ctx(src));
    assert!(diags.is_empty());
}

// ── Message content ───────────────────────────────────────────────────────────

#[test]
fn message_mentions_from() {
    let diags = CommaAfterLastColumn.check(&ctx("SELECT a, b, FROM t"));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("FROM"),
        "message should mention FROM, got: {}",
        diags[0].message
    );
}

#[test]
fn message_mentions_trailing_comma() {
    let diags = CommaAfterLastColumn.check(&ctx("SELECT a, b, FROM t"));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_lowercase().contains("trailing comma"),
        "message should mention trailing comma, got: {}",
        diags[0].message
    );
}

// ── Line and column ───────────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = CommaAfterLastColumn.check(&ctx("SELECT a, b, FROM t"));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── Multiple violations ───────────────────────────────────────────────────────

#[test]
fn two_trailing_commas_two_violations() {
    // Two separate SELECT statements each with a trailing comma before FROM
    let src = "SELECT a, FROM t;\nSELECT b, FROM s";
    let diags = CommaAfterLastColumn.check(&ctx(src));
    assert_eq!(diags.len(), 2);
}

// ── INSERT VALUES — no FROM keyword follows, no violation ────────────────────

#[test]
fn insert_values_no_violation() {
    let diags = CommaAfterLastColumn.check(&ctx("INSERT INTO t VALUES (1, 2)"));
    assert!(diags.is_empty());
}

// ── Parse error resilience ────────────────────────────────────────────────────

#[test]
fn parse_error_still_scans() {
    // Invalid SQL but the comma before FROM should still be detected
    let diags = CommaAfterLastColumn.check(&ctx("SELECT a, FROM FROM"));
    assert_eq!(diags.len(), 1);
}

// ── Subquery ──────────────────────────────────────────────────────────────────

#[test]
fn subquery_trailing_comma_violation() {
    let src = "SELECT * FROM (SELECT a, b, FROM t) sub";
    let diags = CommaAfterLastColumn.check(&ctx(src));
    assert_eq!(diags.len(), 1);
}
