use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::create_schema_statement::CreateSchemaStatement;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CreateSchemaStatement.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CreateSchemaStatement.name(), "Lint/CreateSchemaStatement");
}

#[test]
fn rule_name_starts_with_lint_prefix() {
    assert!(CreateSchemaStatement.name().starts_with("Lint/"));
}

#[test]
fn create_schema_one_violation() {
    let sql = "CREATE SCHEMA myschema";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn drop_schema_no_violation() {
    // DROP SCHEMA is a different rule
    let sql = "DROP SCHEMA myschema";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_table_no_violation() {
    let sql = "CREATE TABLE t (id INT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_view_no_violation() {
    let sql = "CREATE VIEW v AS SELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn case_insensitive_create_schema() {
    // sqlparser is case-insensitive in parsing keywords
    let sql = "create schema myschema";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_schema_in_multi_statement_file() {
    let sql = "SELECT 1;\nCREATE SCHEMA myschema;\nSELECT 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "NOT VALID SQL ###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = CreateSchemaStatement.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn create_schema_if_not_exists_still_flagged() {
    let sql = "CREATE SCHEMA IF NOT EXISTS myschema";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_create_schemas_multiple_violations() {
    let sql = "CREATE SCHEMA s1;\nCREATE SCHEMA s2;\nCREATE SCHEMA s3";
    let diags = check(sql);
    assert_eq!(diags.len(), 3);
}

#[test]
fn message_mentions_dbt_or_configuration() {
    let sql = "CREATE SCHEMA myschema";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("dbt") || msg.contains("configuration") || msg.contains("config"),
        "message should mention dbt or configuration: {}",
        diags[0].message
    );
}

#[test]
fn diagnostic_rule_name_matches() {
    let sql = "CREATE SCHEMA myschema";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Lint/CreateSchemaStatement");
}

#[test]
fn line_col_nonzero() {
    let sql = "CREATE SCHEMA myschema";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn correct_line_for_create_schema() {
    let sql = "SELECT 1;\nCREATE SCHEMA myschema";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}
