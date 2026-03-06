use sqrust_core::FileContext;
use sqrust_rules::layout::leading_operator::LeadingOperator;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    LeadingOperator.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    let sql = "SELECT a\nFROM t\nWHERE x = 1\n  AND y = 2";
    let diags = check(sql);
    assert_eq!(diags[0].rule, "Layout/LeadingOperator");
}

#[test]
fn leading_and_one_violation() {
    let sql = "SELECT a FROM t WHERE a = 1\n  AND b = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn leading_or_one_violation() {
    let sql = "SELECT a FROM t WHERE a = 1\n  OR b = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn trailing_and_no_violation() {
    // AND at end of line — trailing operator style — no violation
    let sql = "SELECT a FROM t WHERE a = 1 AND\n  b = 2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn trailing_or_no_violation() {
    // OR at end of line — trailing operator style — no violation
    let sql = "SELECT a FROM t WHERE a = 1 OR\n  b = 2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn single_line_and_no_violation() {
    // AND on same line — not at start of its own line
    let sql = "SELECT a FROM t WHERE a AND b";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn order_by_no_violation() {
    // ORDER BY starts with OR letters but must NOT be flagged
    let sql = "SELECT a FROM t\nORDER BY name";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_leading_operators_two_violations() {
    // Two ANDs at line start
    let sql = "SELECT a FROM t WHERE x = 1\n  AND y = 2\n  AND z = 3";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn leading_and_and_or_two_violations() {
    // One AND and one OR at line start
    let sql = "SELECT a FROM t WHERE x = 1\n  AND y = 2\n  OR z = 3";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn no_where_no_violation() {
    let sql = "SELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_contains_operator_name() {
    // AND case
    let sql_and = "SELECT a FROM t WHERE x = 1\n  AND y = 2";
    let diags_and = check(sql_and);
    assert!(
        diags_and[0].message.to_uppercase().contains("AND"),
        "Expected message to mention AND, got: {}",
        diags_and[0].message
    );

    // OR case
    let sql_or = "SELECT a FROM t WHERE x = 1\n  OR y = 2";
    let diags_or = check(sql_or);
    assert!(
        diags_or[0].message.to_uppercase().contains("OR"),
        "Expected message to mention OR, got: {}",
        diags_or[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "SELECT a FROM t WHERE x = 1\n  AND y = 2";
    let diags = check(sql);
    assert!(diags[0].line > 0);
    assert!(diags[0].col > 0);
}

#[test]
fn parse_error_still_checks_source() {
    // Even with parse errors, the text-based scan should still run
    let sql = "SELECT a FROM @@bad@@\nWHERE x = 1\n  AND y = 2";
    let ctx = FileContext::from_source(sql, "test.sql");
    let diags = LeadingOperator.check(&ctx);
    assert_eq!(diags.len(), 1);
}

#[test]
fn operator_at_start_with_no_space_but_newline() {
    // AND/OR is the entire line content — edge case
    let sql = "SELECT a FROM t WHERE x = 1\nAND\n  y = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);

    let sql_or = "SELECT a FROM t WHERE x = 1\nOR\n  y = 2";
    let diags_or = check(sql_or);
    assert_eq!(diags_or.len(), 1);
}
