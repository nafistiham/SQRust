use sqrust_core::FileContext;
use sqrust_rules::layout::comment_style::CommentStyle;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    CommentStyle.check(&ctx(sql))
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(CommentStyle.name(), "Layout/CommentStyle");
}

// ── Violations ────────────────────────────────────────────────────────────────

#[test]
fn single_line_block_comment_violation() {
    let diags = check("SELECT 1 /* this is a comment */ FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_inline_block_comments_two_violations() {
    let diags = check("/* c1 */ SELECT /* c2 */ 1");
    assert_eq!(diags.len(), 2);
}

#[test]
fn block_comment_at_start_single_line_violation() {
    let diags = check("/* SQL query */ SELECT 1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn empty_block_comment_single_line_violation() {
    // /**/ has no newline inside — still a single-line block comment
    let diags = check("SELECT /**/ 1");
    assert_eq!(diags.len(), 1);
}

// ── No violations ─────────────────────────────────────────────────────────────

#[test]
fn multiline_block_comment_no_violation() {
    // A block comment that spans two lines is fine — can't be replaced with --
    let diags = check("/* line1\nline2 */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn dash_dash_comment_no_violation() {
    let diags = check("SELECT 1 -- comment");
    assert!(diags.is_empty());
}

#[test]
fn no_comment_no_violation() {
    let diags = check("SELECT 1 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn block_comment_in_string_no_violation() {
    // The /* */ is inside a single-quoted string — not a real comment
    let diags = check("SELECT '/* not a comment */'");
    assert!(diags.is_empty());
}

// ── Message and position ──────────────────────────────────────────────────────

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT /* note */ 1");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    // Should mention -- as the alternative
    assert!(
        msg.contains("--"),
        "message should mention '--', got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT /* note */ 1");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn col_points_to_slash_star() {
    // "SELECT " is 7 chars. The '/' of '/*' is at col 8.
    let diags = check("SELECT /* note */ 1");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 8, "violation col should be at '/' of '/*'");
}

#[test]
fn parse_error_still_checks_source() {
    // Invalid SQL — parser will fail, but text-based check still works
    let diags = check("THIS IS NOT SQL /* inline comment */ !!!");
    assert_eq!(diags.len(), 1, "text-based rule should work despite parse errors");
}
