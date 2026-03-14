use sqrust_core::FileContext;
use sqrust_rules::layout::closing_paren_new_line::ClosingParenNewLine;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ClosingParenNewLine.check(&ctx)
}

// ── name ─────────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    let sql = "SELECT (\n  a,\n  b)";
    let diags = check(sql);
    assert!(!diags.is_empty());
    assert_eq!(diags[0].rule, "Layout/ClosingParenNewLine");
}

// ── violations ───────────────────────────────────────────────────────────────

#[test]
fn closing_paren_after_content_on_same_line_one_violation() {
    // The `)` appears after `b` on a line whose `(` was on an earlier line.
    let sql = "SELECT (\n  a,\n  b)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn closing_paren_inline_with_other_content_flagged() {
    let sql = "SELECT foo(\n  a,\n  b) FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_multi_line_groups_each_missing_own_line_two_violations() {
    let sql = "SELECT (\n  a,\n  b),\n(\n  c,\n  d)";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_is_correct() {
    let sql = "SELECT (\n  a,\n  b)";
    let diags = check(sql);
    assert_eq!(
        diags[0].message,
        "Closing parenthesis of a multi-line expression should be on its own line"
    );
}

#[test]
fn line_and_col_are_nonzero() {
    let sql = "SELECT (\n  a,\n  b)";
    let diags = check(sql);
    assert!(diags[0].line > 0);
    assert!(diags[0].col > 0);
}

#[test]
fn line_points_to_closing_paren_line() {
    // line 3 is "  b)"
    let sql = "SELECT (\n  a,\n  b)";
    let diags = check(sql);
    assert_eq!(diags[0].line, 3);
}

#[test]
fn col_points_to_closing_paren_position() {
    // "  b)" — `)` is at col 4
    let sql = "SELECT (\n  a,\n  b)";
    let diags = check(sql);
    assert_eq!(diags[0].col, 4);
}

// ── no violations ────────────────────────────────────────────────────────────

#[test]
fn closing_paren_on_own_line_no_violation() {
    let sql = "SELECT (\n  a,\n  b\n)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn single_line_paren_no_violation() {
    let sql = "SELECT (a, b) FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn closing_paren_is_first_nonwhitespace_no_violation() {
    // ")" indented — still first non-ws char on line
    let sql = "SELECT (\n  a,\n  b\n  )";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn empty_source_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn paren_inside_string_no_violation() {
    // The parens are inside a string literal — must not be counted.
    let sql = "SELECT '(\nfoo\n)'";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn block_comment_parens_no_violation() {
    // Parens inside a block comment must not be counted.
    let sql = "SELECT 1 /* (\nfoo\n) */";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn line_comment_opening_paren_not_counted() {
    // `(` is in a line comment — should not open a tracked group.
    let sql = "SELECT 1 -- (\nFROM t\n  )";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn nested_multi_line_inner_already_on_own_line_no_violation() {
    // Outer closes on its own line; inner closes on its own line.
    let sql = "SELECT (\n  (\n    a\n  )\n)";
    let diags = check(sql);
    assert!(diags.is_empty());
}
