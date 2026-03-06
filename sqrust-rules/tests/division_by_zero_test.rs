use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::division_by_zero::DivisionByZero;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DivisionByZero.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DivisionByZero.name(), "Ambiguous/DivisionByZero");
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = DivisionByZero.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn integer_zero_divisor_one_violation() {
    let diags = check("SELECT a / 0 FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/DivisionByZero");
}

#[test]
fn non_zero_divisor_no_violation() {
    let diags = check("SELECT a / 1 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn column_divisor_no_violation() {
    let diags = check("SELECT a / b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn float_zero_divisor_one_violation() {
    let diags = check("SELECT a / 0.0 FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multi_digit_zero_float_one_violation() {
    let diags = check("SELECT a / 0.00 FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn division_by_zero_in_where_clause_one_violation() {
    let diags = check("SELECT * FROM t WHERE (price / 0) > 10");
    assert_eq!(diags.len(), 1);
}

#[test]
fn division_by_zero_no_from_one_violation() {
    let diags = check("SELECT 1 / 0");
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_division_no_violation() {
    let diags = check("SELECT a + b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn chained_division_second_is_zero_one_violation() {
    // a / 2 / 0 — only the `/ 0` part is a violation
    let diags = check("SELECT a / 2 / 0 FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_format_is_correct() {
    let diags = check("SELECT a / 0 FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "Division by zero literal; this will cause an error or return NULL"
    );
}

#[test]
fn line_and_col_are_nonzero() {
    let diags = check("SELECT a / 0 FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn indirect_zero_addition_no_violation() {
    // (1 + 0) is not a direct zero literal — the divisor is a BinaryOp, not a Number(0)
    let diags = check("SELECT a / (1 + 0) FROM t");
    assert!(diags.is_empty());
}
