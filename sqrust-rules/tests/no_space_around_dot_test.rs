use sqrust_core::FileContext;
use sqrust_rules::layout::no_space_around_dot::NoSpaceAroundDot;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoSpaceAroundDot.name(), "Layout/NoSpaceAroundDot");
}

// ── Clean (0 violations) ──────────────────────────────────────────────────────

#[test]
fn no_spaces_produces_no_violations() {
    let diags = NoSpaceAroundDot.check(&ctx("SELECT t.col FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn multi_part_no_spaces_produces_no_violations() {
    let diags = NoSpaceAroundDot.check(&ctx("SELECT schema.t.col FROM schema.t"));
    assert!(diags.is_empty());
}

#[test]
fn float_literal_no_spaces_produces_no_violations() {
    let diags = NoSpaceAroundDot.check(&ctx("SELECT 1.5 FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn digit_dot_digit_with_spaces_produces_no_violations() {
    // Edge case: digit-dot-digit even with spaces — skip (both sides digits)
    let diags = NoSpaceAroundDot.check(&ctx("SELECT 1 . 5 FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn dot_in_single_quoted_string_produces_no_violations() {
    let diags = NoSpaceAroundDot.check(&ctx("SELECT 'a . b' FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn dot_in_line_comment_produces_no_violations() {
    let diags = NoSpaceAroundDot.check(&ctx("SELECT a FROM t -- t . col"));
    assert!(diags.is_empty());
}

// ── Violations ────────────────────────────────────────────────────────────────

#[test]
fn space_before_and_after_dot_produces_one_violation() {
    let diags = NoSpaceAroundDot.check(&ctx("SELECT t . col FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn space_before_dot_only_produces_one_violation() {
    let diags = NoSpaceAroundDot.check(&ctx("SELECT t .col FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn space_after_dot_only_produces_one_violation() {
    let diags = NoSpaceAroundDot.check(&ctx("SELECT t. col FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_spaced_dots_produce_three_violations() {
    // schema . t . col → 2 dots with spaces
    // schema . t → 1 more
    // Total: 3
    let diags = NoSpaceAroundDot.check(&ctx("SELECT schema . t . col FROM schema . t"));
    assert_eq!(diags.len(), 3);
}

#[test]
fn mixed_clean_and_violated_produces_correct_count() {
    // t.col is fine, s . col2 is 1 violation
    let diags = NoSpaceAroundDot.check(
        &ctx("SELECT t.col, s . col2 FROM t JOIN s ON t.id = s.id"),
    );
    assert_eq!(diags.len(), 1);
}

// ── Parse errors ──────────────────────────────────────────────────────────────

#[test]
fn parse_error_source_still_runs() {
    // Intentionally broken SQL — rule still checks source
    let diags = NoSpaceAroundDot.check(&ctx("SELECT t . col FROM FROM"));
    assert_eq!(diags.len(), 1);
}

// ── Message content ───────────────────────────────────────────────────────────

#[test]
fn message_mentions_qualified_name_or_dot() {
    let diags = NoSpaceAroundDot.check(&ctx("SELECT t . col FROM t"));
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("qualified name") || msg.contains('.'),
        "message was: {msg}"
    );
}
