use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::set_operator_new_line::SetOperatorNewLine;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    SetOperatorNewLine.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SetOperatorNewLine.name(), "Layout/SetOperatorNewLine");
}

#[test]
fn no_union_no_violation() {
    assert!(check("SELECT id FROM t").is_empty());
}

#[test]
fn union_on_own_line_no_violation() {
    assert!(check("SELECT id FROM t\nUNION ALL\nSELECT id FROM t2").is_empty());
}

#[test]
fn intersect_on_own_line_no_violation() {
    assert!(check("SELECT id FROM t\nINTERSECT\nSELECT id FROM t2").is_empty());
}

#[test]
fn except_on_own_line_no_violation() {
    assert!(check("SELECT id FROM t\nEXCEPT\nSELECT id FROM t2").is_empty());
}

#[test]
fn union_inline_flagged() {
    let d = check("SELECT id FROM t UNION ALL SELECT id FROM t2");
    assert_eq!(d.len(), 1);
}

#[test]
fn union_after_content_on_same_line_flagged() {
    let d = check("SELECT id FROM t UNION\nSELECT id FROM t2");
    assert_eq!(d.len(), 1);
}

#[test]
fn union_before_content_on_same_line_flagged() {
    let d = check("SELECT id FROM t\nUNION SELECT id FROM t2");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_inline_unions_flagged_twice() {
    let d = check("SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_union_or_newline() {
    let d = check("SELECT id FROM t UNION ALL SELECT id FROM t2");
    let msg = d[0].message.to_lowercase();
    assert!(msg.contains("union") || msg.contains("newline") || msg.contains("line"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM t UNION ALL SELECT id FROM t2");
    assert_eq!(d[0].rule, "Layout/SetOperatorNewLine");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM t UNION ALL SELECT id FROM t2");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn union_in_string_not_flagged() {
    assert!(check("SELECT 'UNION ALL' FROM t").is_empty());
}

#[test]
fn union_in_comment_not_flagged() {
    assert!(check("SELECT id FROM t -- UNION ALL\n").is_empty());
}
