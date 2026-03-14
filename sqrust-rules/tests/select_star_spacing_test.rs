use sqrust_core::FileContext;
use sqrust_rules::layout::select_star_spacing::SelectStarSpacing;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelectStarSpacing.name(), "Layout/SelectStarSpacing");
}

// ── No space (Pattern 1) ──────────────────────────────────────────────────────

#[test]
fn select_star_no_space_violation() {
    let diags = SelectStarSpacing.check(&ctx("SELECT* FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_star_no_space_message_content() {
    let diags = SelectStarSpacing.check(&ctx("SELECT* FROM t"));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("SELECT *"),
        "message should suggest 'SELECT *', got: {}",
        diags[0].message
    );
}

// ── Correct spacing — no violation ───────────────────────────────────────────

#[test]
fn select_star_correct_space_no_violation() {
    let diags = SelectStarSpacing.check(&ctx("SELECT * FROM t"));
    assert!(diags.is_empty());
}

// ── Multiple spaces (Pattern 2) ───────────────────────────────────────────────

#[test]
fn select_star_two_spaces_violation() {
    let diags = SelectStarSpacing.check(&ctx("SELECT  * FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_star_many_spaces_violation() {
    let diags = SelectStarSpacing.check(&ctx("SELECT     * FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_star_multiple_spaces_message_content() {
    let diags = SelectStarSpacing.check(&ctx("SELECT  * FROM t"));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("SELECT *"),
        "message should suggest 'SELECT *', got: {}",
        diags[0].message
    );
}

// ── SELECT with column — no violation ────────────────────────────────────────

#[test]
fn select_col_no_violation() {
    let diags = SelectStarSpacing.check(&ctx("SELECT a FROM t"));
    assert!(diags.is_empty());
}

// ── String and comment skipping ───────────────────────────────────────────────

#[test]
fn select_star_in_string_no_violation() {
    let diags = SelectStarSpacing.check(&ctx("SELECT 'SELECT* example' FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn select_star_in_comment_no_violation() {
    let src = "-- SELECT* example\nSELECT a FROM t";
    let diags = SelectStarSpacing.check(&ctx(src));
    assert!(diags.is_empty());
}

// ── Line and column ───────────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = SelectStarSpacing.check(&ctx("SELECT* FROM t"));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── Subquery ──────────────────────────────────────────────────────────────────

#[test]
fn subquery_select_star_no_space_violation() {
    let src = "SELECT * FROM (SELECT* FROM t) sub";
    let diags = SelectStarSpacing.check(&ctx(src));
    assert_eq!(diags.len(), 1);
}

// ── Parse error resilience ────────────────────────────────────────────────────

#[test]
fn parse_error_still_scans() {
    // Invalid SQL but SELECT* should still be detected
    let diags = SelectStarSpacing.check(&ctx("SELECT* FROM FROM"));
    assert_eq!(diags.len(), 1);
}

// ── Case insensitivity ────────────────────────────────────────────────────────

#[test]
fn select_star_case_insensitive() {
    let diags = SelectStarSpacing.check(&ctx("select* FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_star_mixed_case_no_space_violation() {
    let diags = SelectStarSpacing.check(&ctx("Select* FROM t"));
    assert_eq!(diags.len(), 1);
}
