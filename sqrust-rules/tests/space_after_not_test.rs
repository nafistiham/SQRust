use sqrust_core::FileContext;
use sqrust_rules::layout::space_after_not::SpaceAfterNot;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SpaceAfterNot.name(), "Layout/SpaceAfterNot");
}

// ── Basic violations ──────────────────────────────────────────────────────────

#[test]
fn not_directly_before_paren_produces_one_violation() {
    let diags = SpaceAfterNot.check(&ctx("SELECT * FROM t WHERE NOT(col = 1)"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_with_space_before_paren_produces_no_violations() {
    let diags = SpaceAfterNot.check(&ctx("SELECT * FROM t WHERE NOT (col = 1)"));
    assert!(diags.is_empty());
}

#[test]
fn not_without_parens_produces_no_violations() {
    let diags = SpaceAfterNot.check(&ctx("SELECT * FROM t WHERE NOT col = 1"));
    assert!(diags.is_empty());
}

// ── Case insensitivity ────────────────────────────────────────────────────────

#[test]
fn lowercase_not_directly_before_paren_produces_one_violation() {
    let diags = SpaceAfterNot.check(&ctx("SELECT * FROM t WHERE not(col = 1)"));
    assert_eq!(diags.len(), 1);
}

// ── In CASE expression ────────────────────────────────────────────────────────

#[test]
fn not_in_case_when_produces_one_violation() {
    let diags = SpaceAfterNot.check(&ctx(
        "SELECT CASE WHEN NOT(x > 0) THEN 1 END FROM t",
    ));
    assert_eq!(diags.len(), 1);
}

// ── NOT directly followed by ( with no EXISTS ─────────────────────────────────

#[test]
fn not_exists_with_space_produces_no_violations() {
    // NOT followed by space, then EXISTS( — this rule only catches NOT(
    let diags = SpaceAfterNot.check(&ctx(
        "SELECT * FROM t WHERE NOT EXISTS (SELECT 1 FROM s)",
    ));
    assert!(diags.is_empty());
}

#[test]
fn not_exists_without_space_between_not_and_exists_is_a_different_issue() {
    // NOT followed by space before EXISTS, EXISTS has no space before ( — NOT( is not here
    let diags = SpaceAfterNot.check(&ctx(
        "SELECT * FROM t WHERE NOT EXISTS(SELECT 1 FROM s)",
    ));
    // NOT is followed by space then EXISTS( — NOT( doesn't match, so 0 violations for this rule
    assert!(diags.is_empty());
}

// ── In SELECT ─────────────────────────────────────────────────────────────────

#[test]
fn not_in_select_without_space_produces_one_violation() {
    let diags = SpaceAfterNot.check(&ctx("SELECT NOT(1=1) FROM t"));
    assert_eq!(diags.len(), 1);
}

// ── String literals ───────────────────────────────────────────────────────────

#[test]
fn pattern_inside_string_produces_no_violations() {
    let diags = SpaceAfterNot.check(&ctx("SELECT 'NOT(x)' FROM t"));
    assert!(diags.is_empty());
}

// ── Comments ─────────────────────────────────────────────────────────────────

#[test]
fn pattern_inside_line_comment_produces_no_violations() {
    let diags = SpaceAfterNot.check(&ctx("SELECT a FROM t -- NOT(x)"));
    assert!(diags.is_empty());
}

// ── Word boundary ─────────────────────────────────────────────────────────────

#[test]
fn isnot_word_is_not_flagged() {
    // ISNOT( — the "NOT" is part of a longer identifier, not a standalone keyword
    let diags = SpaceAfterNot.check(&ctx("SELECT * FROM t WHERE ISNOT(col)"));
    assert!(diags.is_empty());
}

// ── Multiple violations ───────────────────────────────────────────────────────

#[test]
fn multiple_not_violations_are_all_reported() {
    let diags = SpaceAfterNot.check(&ctx(
        "SELECT * FROM t WHERE NOT(a = 1) AND NOT(b = 2)",
    ));
    assert_eq!(diags.len(), 2);
}

// ── Parse error resilience ────────────────────────────────────────────────────

#[test]
fn parse_error_source_still_runs() {
    let diags = SpaceAfterNot.check(&ctx("SELECT NOT(1=1) FROM FROM"));
    assert_eq!(diags.len(), 1);
}

// ── Message text ──────────────────────────────────────────────────────────────

#[test]
fn violation_message_contains_not_keyword() {
    let diags = SpaceAfterNot.check(&ctx("SELECT * FROM t WHERE NOT(col = 1)"));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("NOT"),
        "message should mention NOT"
    );
}

#[test]
fn violation_message_contains_space_hint() {
    let diags = SpaceAfterNot.check(&ctx("SELECT * FROM t WHERE NOT(col = 1)"));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("NOT ("),
        "message should suggest NOT ( with space"
    );
}
