use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::unsafe_division::UnsafeDivision;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    UnsafeDivision.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnsafeDivision.name(), "Ambiguous/UnsafeDivision");
}

#[test]
fn bare_division_violation() {
    let diags = check("SELECT a / b FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/UnsafeDivision");
}

#[test]
fn nullif_guard_no_violation() {
    let diags = check("SELECT a / NULLIF(b, 0) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nullif_lowercase_guard_no_violation() {
    let diags = check("SELECT a / nullif(b, 0) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn numeric_literal_denominator_no_violation() {
    let diags = check("SELECT a / 2 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn numeric_literal_float_denominator_no_violation() {
    let diags = check("SELECT a / 3.14 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn division_in_string_no_violation() {
    let diags = check("SELECT 'a / b' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn division_in_line_comment_no_violation() {
    let diags = check("-- SELECT a / b\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn division_in_block_comment_no_violation() {
    let diags = check("/* SELECT a / b */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn block_comment_start_slash_star_no_violation() {
    // The '/' in '/*' should not be flagged as division
    let diags = check("/* comment */ SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn multiple_divisions_multiple_violations() {
    let diags = check("SELECT a / b, c / d FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn mixed_guarded_and_unguarded() {
    let diags = check("SELECT a / NULLIF(b, 0), c / d FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn division_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE total / count > 10");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_content() {
    let diags = check("SELECT a / b FROM t");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("nullif") || msg.contains("divide-by-zero"),
        "message should mention NULLIF or divide-by-zero, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT a / b FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn division_on_second_line() {
    let diags = check("SELECT 1,\na / b FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn nullif_with_spaces_no_violation() {
    let diags = check("SELECT a /  NULLIF(b, 0) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn integer_zero_denominator_no_violation() {
    // Dividing by integer literal 0 is flagged differently (DivisionByZero rule)
    // UnsafeDivision only checks for non-literal denominators
    let diags = check("SELECT a / 0 FROM t");
    assert!(diags.is_empty());
}
