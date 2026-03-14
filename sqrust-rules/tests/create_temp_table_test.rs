use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::create_temp_table::CreateTempTable;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CreateTempTable.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CreateTempTable.name(), "Lint/CreateTempTable");
}

#[test]
fn create_temporary_table_violation() {
    let sql = "CREATE TEMPORARY TABLE t (id INT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_temp_table_violation() {
    let sql = "CREATE TEMP TABLE t (id INT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_temporary_table_case_insensitive() {
    let sql = "create temporary table t (id int)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_temp_table_case_insensitive() {
    let sql = "create temp table t (id int)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_table_no_violation() {
    let sql = "CREATE TABLE t (id INT, name TEXT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_from_temp_no_violation() {
    let sql = "SELECT * FROM temp_results WHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_temporary_table_message_content() {
    let sql = "CREATE TEMPORARY TABLE t (id INT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("dialect") || msg.contains("dbt") || msg.contains("ephemeral"),
        "message should mention dialect or dbt: {}",
        diags[0].message
    );
}

#[test]
fn two_create_temp_tables_two_violations() {
    let sql = concat!(
        "CREATE TEMP TABLE t1 (a INT);\n",
        "CREATE TEMPORARY TABLE t2 (b TEXT)",
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn create_temp_table_with_schema_violation() {
    let sql = "CREATE TEMP TABLE schema.t AS SELECT 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn line_nonzero() {
    let sql = "CREATE TEMP TABLE t (id INT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

#[test]
fn col_nonzero() {
    let sql = "CREATE TEMP TABLE t (id INT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn create_temp_table_with_columns_violation() {
    let sql = "CREATE TEMP TABLE t (id INT, name TEXT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_ignored() {
    // Source-level scan always works regardless of parse errors
    let sql = "CREATE TEMP TABLE t (id INT ### bad syntax";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_temp_table_second_line_line_is_two() {
    let sql = concat!(
        "SELECT 1;\n",
        "CREATE TEMP TABLE t (id INT)",
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}
