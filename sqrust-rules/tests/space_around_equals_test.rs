use sqrust_core::FileContext;
use sqrust_rules::layout::space_around_equals::SpaceAroundEquals;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SpaceAroundEquals.name(), "Layout/SpaceAroundEquals");
}

// ── Violations ───────────────────────────────────────────────────────────────

#[test]
fn equals_without_spaces_produces_one_violation() {
    let diags = SpaceAroundEquals.check(&ctx("WHERE col=val"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn equals_missing_space_after_produces_one_violation() {
    let diags = SpaceAroundEquals.check(&ctx("WHERE col =val"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn equals_missing_space_before_produces_one_violation() {
    let diags = SpaceAroundEquals.check(&ctx("WHERE col= val"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn set_clause_without_spaces_produces_one_violation() {
    let diags = SpaceAroundEquals.check(&ctx("SET col=1"));
    assert_eq!(diags.len(), 1);
}

// ── No violations ────────────────────────────────────────────────────────────

#[test]
fn equals_with_spaces_on_both_sides_produces_no_violation() {
    let diags = SpaceAroundEquals.check(&ctx("WHERE col = val"));
    assert!(diags.is_empty());
}

#[test]
fn set_with_spaces_produces_no_violation() {
    let diags = SpaceAroundEquals.check(&ctx("SET col = 1"));
    assert!(diags.is_empty());
}

#[test]
fn not_equals_operator_produces_no_violation() {
    let diags = SpaceAroundEquals.check(&ctx("WHERE col!=val"));
    assert!(diags.is_empty());
}

#[test]
fn less_than_or_equal_produces_no_violation() {
    let diags = SpaceAroundEquals.check(&ctx("WHERE col<=val"));
    assert!(diags.is_empty());
}

#[test]
fn greater_than_or_equal_produces_no_violation() {
    let diags = SpaceAroundEquals.check(&ctx("WHERE col>=val"));
    assert!(diags.is_empty());
}

#[test]
fn diamond_not_equal_produces_no_violation() {
    // <> has no = at all
    let diags = SpaceAroundEquals.check(&ctx("WHERE col<>val"));
    assert!(diags.is_empty());
}

#[test]
fn equals_inside_string_produces_no_violation() {
    // The `=` inside the string literal must not fire
    let diags = SpaceAroundEquals.check(&ctx("WHERE col = 'col=val'"));
    assert!(diags.is_empty());
}

#[test]
fn equals_inside_line_comment_produces_no_violation() {
    let diags = SpaceAroundEquals.check(&ctx("-- col=val"));
    assert!(diags.is_empty());
}

// ── Fix ──────────────────────────────────────────────────────────────────────

#[test]
fn fix_inserts_spaces_around_equals() {
    let c = ctx("col=val");
    let fixed = SpaceAroundEquals.fix(&c).expect("fix should return Some");
    assert_eq!(fixed, "col = val");
}

#[test]
fn fix_returns_none_when_already_spaced() {
    let c = ctx("col = val");
    let result = SpaceAroundEquals.fix(&c);
    assert!(result.is_none());
}

// ── Message ──────────────────────────────────────────────────────────────────

#[test]
fn correct_message_text() {
    let diags = SpaceAroundEquals.check(&ctx("col=val"));
    assert_eq!(diags[0].message, "Operator '=' should have spaces on both sides");
}
