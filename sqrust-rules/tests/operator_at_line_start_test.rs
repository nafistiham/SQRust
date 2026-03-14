use sqrust_core::FileContext;
use sqrust_rules::layout::operator_at_line_start::OperatorAtLineStart;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    OperatorAtLineStart.check(&ctx)
}

// ── name ─────────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    let sql = "SELECT a FROM t WHERE x = 1 AND\n  y = 2";
    let diags = check(sql);
    assert!(!diags.is_empty());
    assert_eq!(diags[0].rule, "Layout/OperatorAtLineStart");
}

// ── AND violations ───────────────────────────────────────────────────────────

#[test]
fn trailing_and_one_violation() {
    let sql = "SELECT a FROM t WHERE x = 1 AND\n  y = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn trailing_and_message_correct() {
    let sql = "SELECT a FROM t WHERE x = 1 AND\n  y = 2";
    let diags = check(sql);
    assert_eq!(
        diags[0].message,
        "AND at end of line; prefer leading operators — move AND to the start of the next line"
    );
}

#[test]
fn trailing_and_lowercase_one_violation() {
    let sql = "SELECT a FROM t WHERE x = 1 and\n  y = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

// ── OR violations ─────────────────────────────────────────────────────────────

#[test]
fn trailing_or_one_violation() {
    let sql = "SELECT a FROM t WHERE x = 1 OR\n  y = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn trailing_or_message_correct() {
    let sql = "SELECT a FROM t WHERE x = 1 OR\n  y = 2";
    let diags = check(sql);
    assert_eq!(
        diags[0].message,
        "OR at end of line; prefer leading operators — move OR to the start of the next line"
    );
}

#[test]
fn trailing_or_lowercase_one_violation() {
    let sql = "SELECT a FROM t WHERE x = 1 or\n  y = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

// ── multiple violations ───────────────────────────────────────────────────────

#[test]
fn two_trailing_and_two_violations() {
    let sql = "SELECT a FROM t WHERE x = 1 AND\n  y = 2 AND\n  z = 3";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn trailing_and_and_or_two_violations() {
    let sql = "SELECT a FROM t WHERE x = 1 AND\n  y = 2 OR\n  z = 3";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

// ── no violations ─────────────────────────────────────────────────────────────

#[test]
fn leading_and_no_violation() {
    // AND at start of line is the `LeadingOperator` rule, not this one.
    let sql = "SELECT a FROM t WHERE x = 1\n  AND y = 2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn leading_or_no_violation() {
    let sql = "SELECT a FROM t WHERE x = 1\n  OR y = 2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn single_line_and_no_violation() {
    let sql = "SELECT a FROM t WHERE a = 1 AND b = 2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn word_ending_in_and_no_violation() {
    // "GRAND" ends with "AND" but is not a trailing AND keyword.
    let sql = "SELECT GRAND\nFROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn word_ending_in_or_no_violation() {
    // "COLOR" ends with "OR" but is not a trailing OR keyword.
    let sql = "SELECT COLOR\nFROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn and_inside_line_comment_no_violation() {
    // -- comment contains AND at end — must not be flagged.
    let sql = "SELECT 1 -- AND\nFROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn line_and_col_are_nonzero() {
    let sql = "SELECT a FROM t WHERE x = 1 AND\n  y = 2";
    let diags = check(sql);
    assert!(diags[0].line > 0);
    assert!(diags[0].col > 0);
}

#[test]
fn col_points_to_start_of_trailing_keyword() {
    // "SELECT a FROM t WHERE x = 1 AND"
    //  col:                          29
    let sql = "SELECT a FROM t WHERE x = 1 AND\n  y = 2";
    let diags = check(sql);
    assert_eq!(diags[0].col, 29);
}
