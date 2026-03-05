use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::negated_is_null::NegatedIsNull;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NegatedIsNull.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NegatedIsNull.name(), "Lint/NegatedIsNull");
}

#[test]
fn not_col_is_null_with_parens_one_violation() {
    let sql = "SELECT * FROM t WHERE NOT (col IS NULL)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_col_is_null_without_parens_one_violation() {
    let sql = "SELECT * FROM t WHERE NOT col IS NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn col_is_not_null_already_correct_no_violation() {
    let sql = "SELECT * FROM t WHERE col IS NOT NULL";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn col_is_null_no_violation() {
    let sql = "SELECT * FROM t WHERE col IS NULL";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn not_qualified_col_is_null_with_parens_one_violation() {
    let sql = "SELECT * FROM t WHERE NOT (t.col IS NULL)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_col_is_not_null_no_violation() {
    let sql = "SELECT * FROM t WHERE NOT (col IS NOT NULL)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn pattern_in_comment_no_violation() {
    let sql = "SELECT * FROM t -- NOT (col IS NULL)\nWHERE 1=1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn pattern_in_string_no_violation() {
    let sql = "SELECT * FROM t WHERE col = 'NOT (col IS NULL)'";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn violation_points_to_not_keyword() {
    let sql = "SELECT * FROM t WHERE NOT (col IS NULL)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    // "NOT" starts at column 23 (1-indexed): "SELECT * FROM t WHERE " is 22 chars
    assert_eq!(diags[0].col, 23);
    assert_eq!(diags[0].line, 1);
}

#[test]
fn message_format_is_correct() {
    let sql = "SELECT * FROM t WHERE NOT col IS NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "Use IS NOT NULL instead of NOT ... IS NULL"
    );
}

#[test]
fn two_violations_on_different_lines() {
    let sql = "SELECT *\nFROM t\nWHERE NOT (a IS NULL)\n  AND NOT (b IS NULL)";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
    assert_eq!(diags[0].line, 3);
    assert_eq!(diags[1].line, 4);
}

#[test]
fn not_col_is_null_no_space_before_paren_one_violation() {
    let sql = "SELECT * FROM t WHERE NOT(col IS NULL)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_followed_by_other_expression_no_violation() {
    // NOT with a boolean expression (not IS NULL pattern)
    let sql = "SELECT * FROM t WHERE NOT (col = 1)";
    let diags = check(sql);
    assert!(diags.is_empty());
}
