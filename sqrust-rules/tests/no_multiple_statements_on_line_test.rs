use sqrust_core::FileContext;
use sqrust_rules::layout::no_multiple_statements_on_line::NoMultipleStatementsOnLine;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(src: &str) -> Vec<sqrust_core::Diagnostic> {
    NoMultipleStatementsOnLine.check(&ctx(src))
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        NoMultipleStatementsOnLine.name(),
        "Layout/NoMultipleStatementsOnLine"
    );
}

// ── No violation cases ────────────────────────────────────────────────────────

#[test]
fn single_statement_no_violation() {
    let diags = check("SELECT 1;");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn two_statements_on_separate_lines_no_violation() {
    let diags = check("SELECT 1;\nSELECT 2;");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn semicolon_then_newline_no_violation() {
    let diags = check("SELECT 1;\n");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn semicolon_then_spaces_then_newline_no_violation() {
    let diags = check("SELECT 1;   \n");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn semicolon_at_eof_no_violation() {
    let diags = check("SELECT 1;");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn semicolon_in_string_not_flagged() {
    // The ';' inside the string literal must not be flagged
    let diags = check("SELECT 'a; b' FROM t;");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn semicolon_in_comment_not_flagged() {
    // ';' after '--' is inside a line comment — skip. The first ';' is
    // followed by ' -- another; stmt' but the next non-whitespace is '--',
    // which starts a comment, so do NOT flag.
    let diags = check("SELECT 1; -- another; stmt");
    assert!(
        diags.is_empty(),
        "expected 0 violations (comment after semicolon), got {}",
        diags.len()
    );
}

// ── Violation cases ───────────────────────────────────────────────────────────

#[test]
fn two_statements_on_same_line_one_violation() {
    let diags = check("SELECT 1; SELECT 2;");
    assert_eq!(diags.len(), 1, "expected 1 violation, got {}", diags.len());
}

#[test]
fn three_on_same_line_two_violations() {
    let diags = check("SELECT 1; SELECT 2; SELECT 3;");
    assert_eq!(diags.len(), 2, "expected 2 violations, got {}", diags.len());
}

// ── Message & position ────────────────────────────────────────────────────────

#[test]
fn violation_message_mentions_line() {
    let diags = check("SELECT 1; SELECT 2;");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_lowercase().contains("statement")
            || diags[0].message.to_lowercase().contains("line"),
        "message should mention statement/line, got: {}",
        diags[0].message
    );
}

#[test]
fn line_nonzero() {
    let diags = check("SELECT 1; SELECT 2;");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line must be >= 1, got {}", diags[0].line);
}

#[test]
fn col_nonzero() {
    let diags = check("SELECT 1; SELECT 2;");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1, "col must be >= 1, got {}", diags[0].col);
}

#[test]
fn col_points_to_start_of_second_statement() {
    // "SELECT 1; SELECT 2;" — 'S' of second SELECT is at col 11 (1-indexed)
    // positions: S=1,E=2,L=3,E=4,C=5,T=6, =7,1=8,;=9, =10,S=11
    let diags = check("SELECT 1; SELECT 2;");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(
        diags[0].col, 11,
        "expected col 11 (start of second SELECT), got {}",
        diags[0].col
    );
}
