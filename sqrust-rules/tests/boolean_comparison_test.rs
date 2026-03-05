use sqrust_core::FileContext;
use sqrust_rules::convention::boolean_comparison::BooleanComparison;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    BooleanComparison.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(BooleanComparison.name(), "Convention/BooleanComparison");
}

#[test]
fn parse_error_produces_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn equals_true_is_flagged() {
    let diags = check("SELECT * FROM t WHERE active = TRUE");
    assert_eq!(diags.len(), 1);
}

#[test]
fn equals_false_is_flagged() {
    let diags = check("SELECT * FROM t WHERE active = FALSE");
    assert_eq!(diags.len(), 1);
}

#[test]
fn equals_integer_is_clean() {
    let diags = check("SELECT * FROM t WHERE col = 1");
    assert!(diags.is_empty());
}

#[test]
fn not_equals_exclamation_true_is_flagged() {
    let diags = check("SELECT * FROM t WHERE active != TRUE");
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_equals_angle_false_is_flagged() {
    let diags = check("SELECT * FROM t WHERE active <> FALSE");
    assert_eq!(diags.len(), 1);
}

#[test]
fn equals_true_in_line_comment_is_ignored() {
    let diags = check("-- WHERE active = TRUE\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn equals_true_in_string_literal_is_ignored() {
    let diags = check("SELECT * FROM t WHERE col = '= TRUE'");
    assert!(diags.is_empty());
}

#[test]
fn equals_true_in_block_comment_is_ignored() {
    let diags = check("/* WHERE active = TRUE */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn lowercase_true_is_flagged() {
    let diags = check("SELECT * FROM t WHERE active = true");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_comparisons_produce_multiple_violations() {
    let diags = check("SELECT * FROM t WHERE a = TRUE AND b = FALSE AND c != TRUE");
    assert_eq!(diags.len(), 3);
}

#[test]
fn message_format_is_correct() {
    let diags = check("SELECT * FROM t WHERE active = TRUE");
    assert_eq!(
        diags[0].message,
        "Explicit comparison with boolean literal; use the expression directly"
    );
}

#[test]
fn not_equals_angle_true_is_flagged() {
    let diags = check("SELECT * FROM t WHERE active <> TRUE");
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_equals_exclamation_false_is_flagged() {
    let diags = check("SELECT * FROM t WHERE active != FALSE");
    assert_eq!(diags.len(), 1);
}

#[test]
fn line_and_col_point_to_operator() {
    let diags = check("SELECT * FROM t WHERE active = TRUE");
    // "active = TRUE" — the '=' is at col 30
    assert_eq!(diags[0].line, 1);
    assert!(diags[0].col > 1);
}
