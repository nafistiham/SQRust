use sqrust_core::FileContext;
use sqrust_rules::layout::parenthesis_spacing::ParenthesisSpacing;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(ParenthesisSpacing.name(), "ParenthesisSpacing");
}

// ── No violations ────────────────────────────────────────────────────────────

#[test]
fn parse_error_produces_no_violations() {
    // Source still has spaces checked by text scan, but this particular input
    // has no paren spacing violations despite a parse error.
    let diags = ParenthesisSpacing.check(&ctx("SELECT FROM FROM"));
    assert!(diags.is_empty());
}

#[test]
fn no_spaces_inside_parens_produces_no_violations() {
    let diags = ParenthesisSpacing.check(&ctx("SELECT (col) FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn func_call_no_spaces_produces_no_violations() {
    let diags = ParenthesisSpacing.check(&ctx("SELECT FUNC(a, b) FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn space_inside_string_literal_produces_no_violations() {
    let diags = ParenthesisSpacing.check(&ctx("SELECT '( a )' FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn space_inside_line_comment_produces_no_violations() {
    let diags = ParenthesisSpacing.check(&ctx("SELECT 1 -- ( a )"));
    assert!(diags.is_empty());
}

#[test]
fn space_inside_block_comment_produces_no_violations() {
    let diags = ParenthesisSpacing.check(&ctx("SELECT 1 /* ( a ) */ FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn newline_after_open_paren_produces_no_violations() {
    let diags = ParenthesisSpacing.check(&ctx("SELECT (\n  col\n) FROM t"));
    assert!(diags.is_empty());
}

// ── Violations ───────────────────────────────────────────────────────────────

#[test]
fn space_after_open_paren_produces_one_violation() {
    let diags = ParenthesisSpacing.check(&ctx("SELECT ( col) FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn space_before_close_paren_produces_one_violation() {
    let diags = ParenthesisSpacing.check(&ctx("SELECT (col ) FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn spaces_on_both_sides_produces_two_violations() {
    let diags = ParenthesisSpacing.check(&ctx("SELECT ( col ) FROM t"));
    assert_eq!(diags.len(), 2);
}

#[test]
fn func_call_with_spaces_produces_two_violations() {
    let diags = ParenthesisSpacing.check(&ctx("SELECT FUNC( a, b ) FROM t"));
    assert_eq!(diags.len(), 2);
}

#[test]
fn two_spaces_after_open_paren_counted_once() {
    // "(  )" — two spaces after `(`: only one violation for the space after `(`
    let diags = ParenthesisSpacing.check(&ctx("SELECT (  ) FROM t"));
    // One violation for space after `(`, one for space before `)`
    assert_eq!(diags.len(), 2);
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[test]
fn message_for_space_after_open_paren_is_correct() {
    let diags = ParenthesisSpacing.check(&ctx("SELECT ( col) FROM t"));
    assert_eq!(diags[0].message, "Space after opening parenthesis; remove the space");
}

#[test]
fn message_for_space_before_close_paren_is_correct() {
    let diags = ParenthesisSpacing.check(&ctx("SELECT (col ) FROM t"));
    assert_eq!(diags[0].message, "Space before closing parenthesis; remove the space");
}

// ── Fix ───────────────────────────────────────────────────────────────────────

#[test]
fn fix_removes_spaces_inside_parens() {
    let c = ctx("SELECT ( col ) FROM t");
    let fixed = ParenthesisSpacing.fix(&c).expect("fix should return Some");
    assert_eq!(fixed, "SELECT (col) FROM t");
}

#[test]
fn fix_preserves_newlines_after_open_paren() {
    // `( ` has a space violation, but the newline after the space should be preserved.
    // Source: "SELECT ( \n  col\n) FROM t"
    // The `(` is followed by a space then a newline; fix removes the space, keeps the newline.
    let c = ctx("SELECT ( \n  col\n) FROM t");
    let fixed = ParenthesisSpacing.fix(&c).expect("fix should return Some");
    assert_eq!(fixed, "SELECT (\n  col\n) FROM t");
}
