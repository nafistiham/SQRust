use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::set_variable_statement::SetVariableStatement;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SetVariableStatement.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SetVariableStatement.name(), "Lint/SetVariableStatement");
}

#[test]
fn set_at_variable_one_violation() {
    let sql = "SET @myvar = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn set_at_variable_case_insensitive_lower() {
    let sql = "set @myvar = 42";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn set_at_variable_case_insensitive_mixed() {
    let sql = "Set @myvar = 'hello'";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn plain_set_no_violation() {
    let sql = "SET search_path = myschema";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_no_violation() {
    let sql = "SELECT * FROM t WHERE a = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn set_in_string_literal_no_violation() {
    let sql = "SELECT 'SET @x = 1' AS example FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn set_in_comment_no_violation() {
    let sql = "-- SET @x = 1\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_dialect() {
    let sql = "SET @myvar = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("dialect") || msg.contains("mysql") || msg.contains("sql server"),
        "message should mention dialect specificity: {}",
        diags[0].message
    );
}

#[test]
fn two_set_at_statements_two_violations() {
    let sql = "SET @a = 1;\nSET @b = 2;";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn line_col_nonzero() {
    let sql = "SET @myvar = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn line_col_second_line() {
    let sql = "SELECT 1;\nSET @x = 99;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn set_word_boundary_no_false_positive() {
    // "OFFSET" contains "SET" but should not match
    let sql = "SELECT * FROM t OFFSET 10";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn set_at_with_expression() {
    let sql = "SET @total = (SELECT COUNT(*) FROM orders)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_still_detects_pattern() {
    let sql = "SET @myvar = ### bad sql";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
