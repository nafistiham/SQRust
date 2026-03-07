use sqrust_core::FileContext;
use sqrust_rules::layout::consistent_comment_style::ConsistentCommentStyle;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    ConsistentCommentStyle.check(&ctx(sql))
}

// ── Rule metadata ──────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(ConsistentCommentStyle.name(), "Layout/ConsistentCommentStyle");
}

// ── No violations ─────────────────────────────────────────────────────────────

#[test]
fn only_line_comments_no_violation() {
    let diags = check("SELECT 1; -- comment\nSELECT 2; -- another");
    assert!(diags.is_empty());
}

#[test]
fn only_block_comments_no_violation() {
    let diags = check("/* comment */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn no_comments_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

#[test]
fn multiline_block_comment_no_violation_alone() {
    let diags = check("/* this is\na multiline comment */\nSELECT 1");
    assert!(diags.is_empty());
}

// ── Violations ────────────────────────────────────────────────────────────────

#[test]
fn mixed_styles_one_violation() {
    // File has both -- and /* */ comments: exactly one violation flagged
    let diags = check("SELECT 1; -- line comment\nSELECT /* block */ 2");
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_styles_flags_minority() {
    // -- appears 5 times, /* */ appears 1 time → flag the /* */ (minority)
    let sql = "SELECT 1; -- a\nSELECT 2; -- b\nSELECT 3; -- c\nSELECT 4; -- d\nSELECT 5; -- e\nSELECT /* block */ 6";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("comment") && (msg.contains("style") || msg.contains("mix")),
        "message should mention 'comment' and 'style' or 'mix', got: {msg}"
    );
    // The /* */ is on the last line (line 6)
    assert_eq!(diags[0].line, 6);
}

#[test]
fn mixed_styles_flags_minority_line_comment() {
    // /* */ appears 5 times, -- appears 1 time → flag the -- (minority)
    let sql = "/* c1 */ SELECT 1;\n/* c2 */ SELECT 2;\n/* c3 */ SELECT 3;\n/* c4 */ SELECT 4;\n/* c5 */ SELECT 5;\nSELECT 6; -- only one";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    // The -- is on line 6
    assert_eq!(diags[0].line, 6);
}

#[test]
fn violation_message_contains_keywords() {
    let diags = check("SELECT 1; -- line\nSELECT /* block */ 2");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("comment"),
        "message should contain 'comment', got: {msg}"
    );
    assert!(
        msg.contains("style") || msg.contains("mix"),
        "message should contain 'style' or 'mix', got: {msg}"
    );
}

#[test]
fn violation_line_is_nonzero() {
    let diags = check("SELECT 1; -- line\nSELECT /* block */ 2");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

// ── String literal handling ────────────────────────────────────────────────────

#[test]
fn block_comment_inside_string_ignored() {
    // '/* not a comment */' is a string literal — should not count as a block comment
    let diags = check("SELECT '/* not a comment */' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn line_comment_after_string_counted() {
    // The -- before the string is inside a string, the second -- is a real comment
    // '-- not a comment' is a string literal; the trailing -- is the real comment
    let diags = check("SELECT '-- not a comment'; -- real comment");
    assert!(diags.is_empty(), "only one comment style: --");
}

// ── Equal count tie-breaking ───────────────────────────────────────────────────

#[test]
fn equal_count_flags_first_of_second_style_seen() {
    // -- appears first, then /* */ appears — same count (1 each)
    // The second style encountered is /* */ — flag its first occurrence
    let sql = "SELECT 1; -- first\nSELECT /* block */ 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    // /* block */ is on line 2
    assert_eq!(diags[0].line, 2);
}
