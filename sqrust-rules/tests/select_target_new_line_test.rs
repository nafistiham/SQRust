use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::select_target_new_line::SelectTargetNewLine;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    SelectTargetNewLine.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelectTargetNewLine.name(), "Layout/SelectTargetNewLine");
}

#[test]
fn single_column_no_violation() {
    assert!(check("SELECT id FROM t").is_empty());
}

#[test]
fn select_star_no_violation() {
    assert!(check("SELECT * FROM t").is_empty());
}

#[test]
fn each_column_on_own_line_no_violation() {
    assert!(check("SELECT\n    id,\n    name\nFROM t").is_empty());
}

#[test]
fn two_columns_same_line_flagged() {
    let d = check("SELECT id, name FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn three_columns_same_line_flagged_once() {
    let d = check("SELECT id, name, email FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn select_on_own_line_cols_on_same_next_line_flagged() {
    let d = check("SELECT\n    id, name\nFROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn message_mentions_column_or_line() {
    let d = check("SELECT id, name FROM t");
    let msg = d[0].message.to_lowercase();
    assert!(msg.contains("column") || msg.contains("line") || msg.contains("select"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id, name FROM t");
    assert_eq!(d[0].rule, "Layout/SelectTargetNewLine");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id, name FROM t");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn comma_in_string_not_counted() {
    assert!(check("SELECT 'a,b' FROM t").is_empty());
}

#[test]
fn function_call_with_comma_single_column_no_violation() {
    assert!(check("SELECT COALESCE(a, b) FROM t").is_empty());
}

#[test]
fn subquery_with_multi_col_select_flagged() {
    let d = check("SELECT * FROM (SELECT id, name FROM t) sub");
    assert_eq!(d.len(), 1);
}

#[test]
fn block_comment_newline_does_not_suppress_flag() {
    // The newline inside the block comment must NOT count as putting columns on separate lines.
    let d = check("SELECT id, /* a comment\n   spanning lines */ name FROM t");
    assert_eq!(d.len(), 1);
}
