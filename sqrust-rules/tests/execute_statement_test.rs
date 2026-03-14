use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::execute_statement::ExecuteStatement;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ExecuteStatement.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ExecuteStatement.name(), "Lint/ExecuteStatement");
}

#[test]
fn execute_one_violation() {
    let sql = "EXECUTE sp_myprocedure";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn exec_one_violation() {
    let sql = "EXEC sp_myprocedure";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn execute_case_insensitive_lower() {
    let sql = "execute sp_myprocedure";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn exec_case_insensitive_mixed() {
    let sql = "Exec sp_foo";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn execute_message_mentions_dialect() {
    let sql = "EXECUTE sp_myprocedure";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("execute") || msg.contains("dialect") || msg.contains("sql server"),
        "message should mention EXECUTE or dialect: {}",
        diags[0].message
    );
}

#[test]
fn exec_message_mentions_sql_server() {
    let sql = "EXEC sp_myprocedure";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("exec") || msg.contains("sql server"),
        "message should mention EXEC or SQL Server: {}",
        diags[0].message
    );
}

#[test]
fn select_no_violation() {
    let sql = "SELECT * FROM t WHERE a = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn execute_in_string_literal_no_violation() {
    let sql = "SELECT 'EXECUTE sp_foo' AS example FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn execute_in_comment_no_violation() {
    let sql = "-- EXECUTE sp_foo\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn exec_in_string_literal_no_violation() {
    let sql = "SELECT 'EXEC sp_foo' AS example FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_executes_two_violations() {
    let sql = "EXECUTE sp_one;\nEXECUTE sp_two;";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn execute_and_exec_two_violations() {
    let sql = "EXECUTE sp_one;\nEXEC sp_two;";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn line_col_nonzero() {
    let sql = "EXECUTE sp_myprocedure";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn line_col_second_line() {
    let sql = "SELECT 1;\nEXECUTE sp_foo;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn execute_word_boundary_no_false_positive() {
    // "EXECUTE_proc" should not match due to underscore after
    let sql = "SELECT EXECUTE_FLAG FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}
