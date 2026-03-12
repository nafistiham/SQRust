use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::space_around_concat_operator::SpaceAroundConcatOperator;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SpaceAroundConcatOperator.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        SpaceAroundConcatOperator.name(),
        "Layout/SpaceAroundConcatOperator"
    );
}

#[test]
fn correct_spacing_no_violations() {
    let diags = check("SELECT a || b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_spaces_one_violation() {
    let diags = check("SELECT a||b FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_space_after_one_violation() {
    let diags = check("SELECT a ||b FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_space_before_one_violation() {
    let diags = check("SELECT a|| b FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_concat_all_correct_no_violations() {
    let diags = check("SELECT a || b || c FROM t");
    assert!(diags.is_empty());
}

#[test]
fn multiple_concat_all_wrong_two_violations() {
    let diags = check("SELECT a||b||c FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn no_concat_operator_no_violations() {
    let diags = check("SELECT a, b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn concat_inside_string_literal_no_violations() {
    let diags = check("SELECT a || b FROM t WHERE x = 'a||b'");
    assert!(diags.is_empty());
}

#[test]
fn concat_inside_line_comment_no_violations() {
    let diags = check("SELECT a || b FROM t -- a||b in comment");
    assert!(diags.is_empty());
}

#[test]
fn concat_inside_block_comment_no_violations() {
    let diags = check("SELECT a || b FROM t /* a||b in block comment */");
    assert!(diags.is_empty());
}

#[test]
fn coalesce_with_spaces_no_violations() {
    let diags = check("SELECT COALESCE(a,'') || COALESCE(b,'') FROM t");
    assert!(diags.is_empty());
}

#[test]
fn coalesce_without_spaces_one_violation() {
    let diags = check("SELECT COALESCE(a,'')||COALESCE(b,'') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn source_level_rule_works_regardless_of_parse_errors() {
    // This SQL doesn't parse cleanly but the rule is source-level
    let diags = check("SELECTFROM a||b");
    assert_eq!(diags.len(), 1);
}

#[test]
fn violation_message_is_correct() {
    let diags = check("SELECT a||b FROM t");
    assert_eq!(
        diags[0].message,
        "Missing space around || concat operator — use 'a || b' style"
    );
}

#[test]
fn violation_rule_name_is_correct() {
    let diags = check("SELECT a||b FROM t");
    assert_eq!(diags[0].rule, "Layout/SpaceAroundConcatOperator");
}
