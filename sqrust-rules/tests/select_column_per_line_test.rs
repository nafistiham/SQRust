use sqrust_core::FileContext;
use sqrust_rules::layout::select_column_per_line::SelectColumnPerLine;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelectColumnPerLine.name(), "Layout/SelectColumnPerLine");
}

// ── No violation: one column per line ─────────────────────────────────────────

#[test]
fn single_column_per_line_no_violation() {
    let sql = "SELECT\n  a,\n  b,\n  c\nFROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn select_star_no_violation() {
    let sql = "SELECT * FROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn select_single_column_no_violation() {
    let sql = "SELECT a FROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn select_one_column_multiline_no_violation() {
    let sql = "SELECT\n  a\nFROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn select_1_no_violation() {
    let sql = "SELECT 1";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Violations: multiple columns on one line ──────────────────────────────────

#[test]
fn two_columns_on_same_line_in_multiline_select_violation() {
    // SELECT list spans 2 lines (col on line after SELECT), but one line has comma
    let sql = "SELECT\n  a, b\nFROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_columns_mixed_lines_violation() {
    let sql = "SELECT\n  a, b,\n  c\nFROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_lines_each_with_commas_violation() {
    let sql = "SELECT\n  a, b,\n  c, d\nFROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert_eq!(diags.len(), 2);
}

#[test]
fn case_insensitive_select_keyword() {
    let sql = "select\n  a, b\nfrom t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_insensitive_from_keyword() {
    let sql = "SELECT\n  a, b\nFROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

// ── Skip strings and comments ──────────────────────────────────────────────────

#[test]
fn comma_in_string_in_select_list_no_violation() {
    let sql = "SELECT\n  'a, b'\nFROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn comma_in_line_comment_in_select_list_no_violation() {
    let sql = "SELECT\n  a -- a, b\nFROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn comma_in_block_comment_no_violation() {
    let sql = "SELECT\n  a /* a, b */\nFROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Edge cases ────────────────────────────────────────────────────────────────

#[test]
fn empty_source_no_violation() {
    let diags = SelectColumnPerLine.check(&ctx(""));
    assert!(diags.is_empty());
}

#[test]
fn no_select_no_violation() {
    let sql = "UPDATE t SET a = 1 WHERE id = 2";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn select_with_only_last_column_comma_trailing_no_violation() {
    // Trailing comma at end of line (last column before FROM) — no second column
    let sql = "SELECT\n  a,\nFROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Message and position ──────────────────────────────────────────────────────

#[test]
fn violation_message_mentions_columns() {
    let sql = "SELECT\n  a, b\nFROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("column"),
        "message should mention 'column', got: {}",
        diags[0].message
    );
}

#[test]
fn violation_line_is_correct() {
    // "a, b" is on line 2
    let sql = "SELECT\n  a, b\nFROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn violation_col_is_nonzero() {
    let sql = "SELECT\n  a, b\nFROM t";
    let diags = SelectColumnPerLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col > 0);
}
