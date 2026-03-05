use sqrust_core::FileContext;
use sqrust_rules::layout::comment_spacing::CommentSpacing;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(CommentSpacing.name(), "Layout/CommentSpacing");
}

// ── Violations ───────────────────────────────────────────────────────────────

#[test]
fn comment_without_space_produces_one_violation() {
    let diags = CommentSpacing.check(&ctx("SELECT 1 --comment"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn comment_at_start_of_line_without_space_produces_one_violation() {
    let diags = CommentSpacing.check(&ctx("--comment\nSELECT 1"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_violations_on_different_lines() {
    let diags = CommentSpacing.check(&ctx("--first\nSELECT 1\n--second"));
    assert_eq!(diags.len(), 2);
}

// ── No violations ────────────────────────────────────────────────────────────

#[test]
fn comment_with_space_produces_no_violation() {
    let diags = CommentSpacing.check(&ctx("SELECT 1 -- comment"));
    assert!(diags.is_empty());
}

#[test]
fn empty_comment_produces_no_violation() {
    // Nothing after --  (end of input)
    let diags = CommentSpacing.check(&ctx("SELECT 1 --"));
    assert!(diags.is_empty());
}

#[test]
fn empty_comment_with_newline_produces_no_violation() {
    // `--` immediately followed by newline
    let diags = CommentSpacing.check(&ctx("SELECT 1 --\nSELECT 2"));
    assert!(diags.is_empty());
}

#[test]
fn triple_dash_divider_is_exempt() {
    let diags = CommentSpacing.check(&ctx("SELECT 1 ---"));
    assert!(diags.is_empty());
}

#[test]
fn many_dash_divider_is_exempt() {
    let diags = CommentSpacing.check(&ctx("SELECT 1 ---- divider ----"));
    assert!(diags.is_empty());
}

#[test]
fn double_dash_inside_single_quoted_string_is_not_a_comment() {
    let diags = CommentSpacing.check(&ctx("SELECT '--notacomment'"));
    assert!(diags.is_empty());
}

#[test]
fn comment_on_its_own_line_with_space_produces_no_violation() {
    let diags = CommentSpacing.check(&ctx("-- comment\nSELECT 1"));
    assert!(diags.is_empty());
}

#[test]
fn double_dash_inside_block_comment_produces_no_violation() {
    let diags = CommentSpacing.check(&ctx("/*--inside block comment*/"));
    assert!(diags.is_empty());
}

// ── Fix ──────────────────────────────────────────────────────────────────────

#[test]
fn fix_inserts_space_after_double_dash() {
    let c = ctx("--note");
    let fixed = CommentSpacing.fix(&c).expect("fix should return Some");
    assert_eq!(fixed, "-- note");
}

#[test]
fn fix_returns_none_when_no_changes_needed() {
    let c = ctx("-- already");
    let result = CommentSpacing.fix(&c);
    assert!(result.is_none());
}

// ── Message ──────────────────────────────────────────────────────────────────

#[test]
fn correct_message_text() {
    let diags = CommentSpacing.check(&ctx("--note"));
    assert_eq!(
        diags[0].message,
        "Line comment should have a space after '--'; write '-- comment'"
    );
}
