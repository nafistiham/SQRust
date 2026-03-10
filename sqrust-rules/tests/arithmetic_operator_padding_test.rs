use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::arithmetic_operator_padding::ArithmeticOperatorPadding;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    ArithmeticOperatorPadding.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ArithmeticOperatorPadding.name(), "Layout/ArithmeticOperatorPadding");
}

#[test]
fn spaced_plus_no_violation() {
    assert!(check("SELECT a + b FROM t").is_empty());
}

#[test]
fn spaced_minus_no_violation() {
    assert!(check("SELECT a - b FROM t").is_empty());
}

#[test]
fn select_star_no_violation() {
    assert!(check("SELECT * FROM t").is_empty());
}

#[test]
fn count_star_no_violation() {
    assert!(check("SELECT COUNT(*) FROM t").is_empty());
}

#[test]
fn unspaced_plus_flagged() {
    let d = check("SELECT a+b FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn unspaced_minus_flagged() {
    let d = check("SELECT a-b FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn unspaced_multiply_flagged() {
    let d = check("SELECT price*1.1 FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn unspaced_divide_flagged() {
    let d = check("SELECT total/count FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn unspaced_modulo_flagged() {
    let d = check("SELECT id%2 FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn operator_in_string_not_flagged() {
    assert!(check("SELECT 'a+b' FROM t").is_empty());
}

#[test]
fn line_comment_operator_not_flagged() {
    assert!(check("SELECT id FROM t -- a+b\nWHERE id > 1").is_empty());
}

#[test]
fn message_mentions_space_or_padding() {
    let d = check("SELECT a+b FROM t");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("space") || msg.contains("pad") || msg.contains("operator"),
        "expected message to mention space/padding/operator, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT a+b FROM t");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Layout/ArithmeticOperatorPadding");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT a+b FROM t");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}
