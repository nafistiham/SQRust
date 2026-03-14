use sqrust_core::FileContext;
use sqrust_rules::convention::no_minus_operator::NoMinusOperator;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoMinusOperator.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoMinusOperator.name(), "Convention/NoMinusOperator");
}

#[test]
fn minus_on_own_line_violation() {
    let sql = "SELECT a FROM t1\nMINUS\nSELECT a FROM t2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn minus_lowercase_on_own_line_violation() {
    let sql = "SELECT a FROM t1\nminus\nSELECT a FROM t2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn minus_mixed_case_on_own_line_violation() {
    let sql = "SELECT a FROM t1\nMinus\nSELECT a FROM t2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn minus_all_on_own_line_violation() {
    let sql = "SELECT a FROM t1\nMINUS ALL\nSELECT a FROM t2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn minus_with_leading_whitespace_violation() {
    let sql = "SELECT a FROM t1\n  MINUS\nSELECT a FROM t2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn except_no_violation() {
    let sql = "SELECT a FROM t1\nEXCEPT\nSELECT a FROM t2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn arithmetic_minus_no_violation() {
    // Arithmetic subtraction: col - 1 should not be flagged
    let diags = check("SELECT col - 1 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn unary_minus_no_violation() {
    let diags = check("SELECT -1 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn minus_in_string_no_violation() {
    let diags = check("SELECT 'MINUS' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn minus_in_comment_no_violation() {
    let sql = "-- use MINUS for set difference\nSELECT col FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn minus_in_identifier_no_violation() {
    // Column named minus_value should not be flagged
    let diags = check("SELECT minus_value FROM t");
    assert!(diags.is_empty());
}

#[test]
fn minus_all_lowercase_violation() {
    let sql = "SELECT a FROM t1\nminus all\nSELECT a FROM t2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_minus_and_except() {
    let sql = "SELECT a FROM t1\nMINUS\nSELECT a FROM t2";
    let diags = check(sql);
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("MINUS"),
        "message should contain 'MINUS', got: {msg}"
    );
    assert!(
        upper.contains("EXCEPT"),
        "message should mention EXCEPT, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "SELECT a FROM t1\nMINUS\nSELECT a FROM t2";
    let diags = check(sql);
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn minus_line_number_correct() {
    // MINUS is on line 2
    let sql = "SELECT a FROM t1\nMINUS\nSELECT a FROM t2";
    let diags = check(sql);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn inline_minus_no_violation() {
    // MINUS appearing inline (not the only token on a trimmed line) should not be flagged
    let diags = check("SELECT a MINUS b FROM t");
    assert!(diags.is_empty());
}
