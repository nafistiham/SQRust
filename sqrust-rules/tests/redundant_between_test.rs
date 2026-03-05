use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::redundant_between::RedundantBetween;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    RedundantBetween.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(RedundantBetween.name(), "Ambiguous/RedundantBetween");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = RedundantBetween.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn different_numeric_bounds_no_violation() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN 1 AND 5");
    assert!(diags.is_empty());
}

#[test]
fn same_numeric_bounds_one_violation() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN 5 AND 5");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/RedundantBetween");
}

#[test]
fn same_string_bounds_one_violation() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN 'a' AND 'a'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn different_string_bounds_no_violation() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN 'a' AND 'b'");
    assert!(diags.is_empty());
}

#[test]
fn not_between_same_bounds_one_violation() {
    let diags = check("SELECT * FROM t WHERE col NOT BETWEEN 5 AND 5");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("!="),
        "expected '!=' in message for NOT BETWEEN, got: {}",
        diags[0].message
    );
}

#[test]
fn two_betweens_only_second_flagged() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN 1 AND 2 AND other BETWEEN 3 AND 3");
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_between_no_violation() {
    let diags = check("SELECT * FROM t WHERE col = 5");
    assert!(diags.is_empty());
}

#[test]
fn different_identifier_bounds_no_violation() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN x AND y");
    assert!(diags.is_empty());
}

#[test]
fn same_identifier_bounds_one_violation() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN x AND x");
    assert_eq!(diags.len(), 1);
}

#[test]
fn between_message_format_correct() {
    let diags = check("SELECT * FROM t WHERE col BETWEEN 5 AND 5");
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "BETWEEN with identical bounds; use = instead"
    );
}

#[test]
fn not_between_message_format_correct() {
    let diags = check("SELECT * FROM t WHERE col NOT BETWEEN 5 AND 5");
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "NOT BETWEEN with identical bounds; use != instead"
    );
}

#[test]
fn line_col_points_to_between_keyword() {
    // "SELECT * FROM t WHERE col BETWEEN 5 AND 5"
    // BETWEEN starts at byte offset 26 (0-indexed), col=27
    let diags = check("SELECT * FROM t WHERE col BETWEEN 5 AND 5");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 27);
}

#[test]
fn select_without_where_no_violation() {
    let diags = check("SELECT col FROM t");
    assert!(diags.is_empty());
}
