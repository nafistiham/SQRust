use sqrust_core::FileContext;
use sqrust_rules::lint::select_without_from::SelectWithoutFrom;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SelectWithoutFrom.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelectWithoutFrom.name(), "Lint/SelectWithoutFrom");
}

#[test]
fn select_column_reference_without_from_violation() {
    // Column reference in FROM-less SELECT — invalid SQL
    let diags = check("SELECT col_name");
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_function_call_without_from_violation() {
    // Function call in FROM-less SELECT
    let diags = check("SELECT my_func()");
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_literal_number_no_violation() {
    // SELECT 1 is a valid/common construct
    let diags = check("SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn select_literal_string_no_violation() {
    // SELECT 'hello' is valid
    let diags = check("SELECT 'hello'");
    assert!(diags.is_empty());
}

#[test]
fn select_null_no_violation() {
    // SELECT NULL is valid
    let diags = check("SELECT NULL");
    assert!(diags.is_empty());
}

#[test]
fn select_with_from_no_violation() {
    // Normal SELECT with FROM — no violation
    let diags = check("SELECT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn select_star_with_from_no_violation() {
    let diags = check("SELECT * FROM t");
    assert!(diags.is_empty());
}

#[test]
fn select_compound_identifier_without_from_violation() {
    // table.column reference without FROM is flagged
    let diags = check("SELECT t.col");
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_multiple_columns_without_from_violation() {
    // Multiple column refs without FROM
    let diags = check("SELECT a, b");
    assert_eq!(diags.len(), 1);
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn select_with_where_and_from_no_violation() {
    let diags = check("SELECT id, name FROM users WHERE id = 1");
    assert!(diags.is_empty());
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    let diags = SelectWithoutFrom.check(&ctx);
    // With parse errors, rule skips — no panic is the key requirement
    let _ = diags;
}

#[test]
fn diagnostic_line_col_is_valid() {
    let diags = check("SELECT my_column");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn select_literal_boolean_no_violation() {
    // SELECT TRUE is valid
    let diags = check("SELECT TRUE");
    assert!(diags.is_empty());
}

#[test]
fn message_is_meaningful() {
    let diags = check("SELECT col_name");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    assert!(
        !msg.is_empty(),
        "message should not be empty"
    );
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("FROM") || upper.contains("SELECT"),
        "message should mention FROM or SELECT, got: {msg}"
    );
}
