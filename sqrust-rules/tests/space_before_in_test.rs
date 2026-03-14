use sqrust_core::FileContext;
use sqrust_rules::layout::space_before_in::SpaceBeforeIn;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SpaceBeforeIn.name(), "Layout/SpaceBeforeIn");
}

// ── Basic violations ──────────────────────────────────────────────────────────

#[test]
fn in_directly_before_paren_produces_one_violation() {
    let diags = SpaceBeforeIn.check(&ctx("SELECT * FROM t WHERE col IN(1, 2, 3)"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn in_with_space_before_paren_produces_no_violations() {
    let diags = SpaceBeforeIn.check(&ctx("SELECT * FROM t WHERE col IN (1, 2, 3)"));
    assert!(diags.is_empty());
}

#[test]
fn not_in_without_space_produces_one_violation() {
    let diags = SpaceBeforeIn.check(&ctx("SELECT * FROM t WHERE col NOT IN(1, 2)"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_in_with_space_produces_no_violations() {
    let diags = SpaceBeforeIn.check(&ctx("SELECT * FROM t WHERE col NOT IN (1, 2)"));
    assert!(diags.is_empty());
}

// ── Subquery ──────────────────────────────────────────────────────────────────

#[test]
fn in_subquery_without_space_produces_one_violation() {
    let diags = SpaceBeforeIn.check(&ctx(
        "SELECT * FROM t WHERE col IN(SELECT id FROM s)",
    ));
    assert_eq!(diags.len(), 1);
}

#[test]
fn in_subquery_with_space_produces_no_violations() {
    let diags = SpaceBeforeIn.check(&ctx(
        "SELECT * FROM t WHERE col IN (SELECT id FROM s)",
    ));
    assert!(diags.is_empty());
}

// ── Case insensitivity ────────────────────────────────────────────────────────

#[test]
fn lowercase_in_directly_before_paren_produces_one_violation() {
    let diags = SpaceBeforeIn.check(&ctx("SELECT * FROM t WHERE col in(1)"));
    assert_eq!(diags.len(), 1);
}

// ── String literals ───────────────────────────────────────────────────────────

#[test]
fn pattern_inside_string_produces_no_violations() {
    let diags = SpaceBeforeIn.check(&ctx("SELECT 'IN(1,2)' FROM t"));
    assert!(diags.is_empty());
}

// ── Comments ─────────────────────────────────────────────────────────────────

#[test]
fn pattern_inside_line_comment_produces_no_violations() {
    let diags = SpaceBeforeIn.check(&ctx("SELECT a FROM t -- col IN(1,2)"));
    assert!(diags.is_empty());
}

// ── Word boundary — IN must be a complete word ────────────────────────────────

#[test]
fn join_table_name_not_flagged() {
    // 'join_table' contains no standalone IN word
    let diags = SpaceBeforeIn.check(&ctx("SELECT a FROM join_table"));
    assert!(diags.is_empty());
}

#[test]
fn inner_join_not_flagged() {
    // INNER starts with IN but INNER is a longer word
    let diags = SpaceBeforeIn.check(&ctx(
        "SELECT a FROM t INNER JOIN s ON t.id = s.id",
    ));
    assert!(diags.is_empty());
}

// ── Multiple violations ───────────────────────────────────────────────────────

#[test]
fn multiple_in_violations_are_all_reported() {
    let diags = SpaceBeforeIn.check(&ctx(
        "SELECT * FROM t WHERE a IN(1,2) AND b IN(3,4)",
    ));
    assert_eq!(diags.len(), 2);
}

// ── Parse error resilience ────────────────────────────────────────────────────

#[test]
fn parse_error_source_still_runs() {
    let diags = SpaceBeforeIn.check(&ctx("SELECT col IN(1,2) FROM FROM"));
    assert_eq!(diags.len(), 1);
}

// ── Message text ──────────────────────────────────────────────────────────────

#[test]
fn violation_message_contains_in_keyword() {
    let diags = SpaceBeforeIn.check(&ctx("SELECT * FROM t WHERE col IN(1, 2, 3)"));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("IN"),
        "message should mention IN"
    );
}

#[test]
fn violation_message_contains_space_hint() {
    let diags = SpaceBeforeIn.check(&ctx("SELECT * FROM t WHERE col IN(1, 2, 3)"));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("IN ("),
        "message should suggest IN ( with space"
    );
}
