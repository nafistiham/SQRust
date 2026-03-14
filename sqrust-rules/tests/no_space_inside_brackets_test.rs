use sqrust_core::FileContext;
use sqrust_rules::layout::no_space_inside_brackets::NoSpaceInsideBrackets;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoSpaceInsideBrackets.name(), "Layout/NoSpaceInsideBrackets");
}

// ── Basic violations ──────────────────────────────────────────────────────────

#[test]
fn space_after_open_bracket_violation() {
    // "[ col ]" has both a space after [ and a space before ]
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT [ col ] FROM t"));
    assert!(!diags.is_empty(), "expected violations for '[ col ]'");
}

#[test]
fn no_space_no_violation() {
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT [col] FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn space_before_close_bracket_violation() {
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT [col ] FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn bracket_with_content_no_extra_space_no_violation() {
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT [dbo].[table] FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn multiple_bracketed_cols_multiple_violations() {
    // "[ a ]" has 2 violations, "[ b ]" has 2 violations → 4 total
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT [ a ], [ b ] FROM t"));
    assert!(diags.len() >= 2, "expected at least 2 violations");
}

#[test]
fn bracket_in_string_no_violation() {
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT '[ not a bracket ]' FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn regular_identifiers_no_violation() {
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT col FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn bracket_table_name_no_violation() {
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT * FROM [dbo].[table]"));
    assert!(diags.is_empty());
}

#[test]
fn bracket_space_after_violation_line_col() {
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT [ col] FROM t"));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn empty_brackets_no_violation() {
    // Edge case: [] has no space inside, no violation.
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT [] FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn parse_error_still_scans() {
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT [ col ] FROM FROM"));
    assert!(!diags.is_empty(), "source-level scan should work despite parse error");
}

#[test]
fn message_content_open() {
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT [ col] FROM t"));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("["),
        "message should mention '['"
    );
}

#[test]
fn message_content_close() {
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT [col ] FROM t"));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("]"),
        "message should mention ']'"
    );
}

#[test]
fn nested_brackets_violation() {
    // "[ schema ].[ table ]" → violations for spaces inside each bracket pair
    let diags = NoSpaceInsideBrackets.check(&ctx("SELECT [ schema ].[ table ] FROM t"));
    assert!(diags.len() >= 2, "expected violations for spaces in both bracket pairs");
}
