use sqrust_core::FileContext;
use sqrust_rules::layout::no_space_after_unary_minus::NoSpaceAfterUnaryMinus;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoSpaceAfterUnaryMinus.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoSpaceAfterUnaryMinus.name(), "Layout/NoSpaceAfterUnaryMinus");
}

#[test]
fn no_space_after_minus_in_select_no_violation() {
    // -col immediately after open paren, no space
    let diags = check("SELECT (-col + 5) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn space_after_minus_after_open_paren_one_violation() {
    let diags = check("SELECT (- col + 5) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_space_after_equals_minus_no_violation() {
    let diags = check("WHERE x = -5");
    assert!(diags.is_empty());
}

#[test]
fn space_after_minus_after_equals_one_violation() {
    let diags = check("WHERE x = - 5");
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_space_after_minus_column_no_violation() {
    let diags = check("WHERE x = -col");
    assert!(diags.is_empty());
}

#[test]
fn no_space_in_paren_unary_no_violation() {
    let diags = check("SELECT a + (-col) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn space_after_unary_minus_in_paren_one_violation() {
    let diags = check("SELECT a + (- col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn minus_in_string_not_flagged() {
    let diags = check("SELECT '- col' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn minus_in_line_comment_not_flagged() {
    let diags = check("SELECT a FROM t -- - col");
    assert!(diags.is_empty());
}

#[test]
fn minus_in_block_comment_not_flagged() {
    let diags = check("SELECT /* - col */ a FROM t");
    assert!(diags.is_empty());
}

#[test]
fn space_after_minus_after_greater_than_one_violation() {
    let diags = check("WHERE x > - 5");
    assert_eq!(diags.len(), 1);
}

#[test]
fn space_after_minus_after_comma_one_violation() {
    let diags = check("SELECT func(a, - b) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_violations_correct_count() {
    // Two unary minus with spaces: (- a) and (- b)
    let diags = check("SELECT (- a) + (- b) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_contains_unary_minus_description() {
    let diags = check("WHERE x = - 5");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("unary minus"),
        "message should mention 'unary minus'"
    );
}
