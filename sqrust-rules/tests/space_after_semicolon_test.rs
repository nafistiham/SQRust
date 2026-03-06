use sqrust_core::FileContext;
use sqrust_rules::layout::space_after_semicolon::SpaceAfterSemicolon;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(src: &str) -> Vec<sqrust_core::Diagnostic> {
    SpaceAfterSemicolon.check(&ctx(src))
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SpaceAfterSemicolon.name(), "Layout/SpaceAfterSemicolon");
}

// ── No violation cases ────────────────────────────────────────────────────────

#[test]
fn semicolon_at_end_of_line_no_violation() {
    // Each ';' is at end of its line
    let diags = check("SELECT 1;\nSELECT 2;");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn semicolon_followed_by_newline_no_violation() {
    let diags = check("SELECT 1;\n");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn semicolon_followed_by_comment_no_violation() {
    // ';' followed by whitespace then '--' comment — ok
    let diags = check("SELECT 1; -- comment\nSELECT 2;");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn semicolon_at_eof_no_violation() {
    let diags = check("SELECT 1;");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn semicolon_inside_string_no_violation() {
    // The ';' inside the string literal must not be flagged
    let diags = check("SELECT 'a;b' FROM t;");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn no_semicolon_no_violation() {
    let diags = check("SELECT 1");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn whitespace_after_semicolon_then_newline_no_violation() {
    // Trailing spaces after ';' before newline — ok
    let diags = check("SELECT 1;   \nSELECT 2");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

// ── Violation cases ───────────────────────────────────────────────────────────

#[test]
fn semicolon_followed_by_content_one_violation() {
    // ';' followed immediately by more SQL on the same line
    let diags = check("SELECT 1; SELECT 2");
    assert_eq!(diags.len(), 1, "expected 1 violation, got {}", diags.len());
}

#[test]
fn two_violations_two_semicolons() {
    // Two semicolons each followed by more SQL on the same line
    let diags = check("SELECT 1; SELECT 2; SELECT 3");
    assert_eq!(diags.len(), 2, "expected 2 violations, got {}", diags.len());
}

// ── Message & position ────────────────────────────────────────────────────────

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT 1; SELECT 2");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_lowercase().contains("semicolon")
            || diags[0].message.to_lowercase().contains("newline")
            || diags[0].message.to_lowercase().contains("statement"),
        "message should mention semicolon/newline/statement, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT 1; SELECT 2");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line must be >= 1");
    assert!(diags[0].col >= 1, "col must be >= 1");
}

#[test]
fn violation_col_is_semicolon_position() {
    // "SELECT 1; SELECT 2" — ';' is at col 9 (1-indexed)
    let diags = check("SELECT 1; SELECT 2");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 9, "expected col 9 (position of ';'), got {}", diags[0].col);
}
