use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::create_or_replace::CreateOrReplace;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CreateOrReplace.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CreateOrReplace.name(), "Lint/CreateOrReplace");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "CREATE OR GARBAGE @@@###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = CreateOrReplace.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn create_or_replace_view_one_violation() {
    let sql = "CREATE OR REPLACE VIEW v AS SELECT 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_view_no_or_replace_no_violation() {
    let sql = "CREATE VIEW v AS SELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_or_replace_table_violation() {
    // sqlparser 0.53 GenericDialect supports CREATE OR REPLACE TABLE
    let sql = "CREATE OR REPLACE TABLE t (id INT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_table_no_violation() {
    let sql = "CREATE TABLE t (id INT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_table_if_not_exists_no_violation() {
    let sql = "CREATE TABLE IF NOT EXISTS t (id INT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_no_violation() {
    let sql = "SELECT id, name FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_or_replace_two_violations() {
    let sql = "CREATE OR REPLACE VIEW v1 AS SELECT 1;\nCREATE OR REPLACE VIEW v2 AS SELECT 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_contains_useful_text() {
    let sql = "CREATE OR REPLACE VIEW v AS SELECT 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_lowercase().contains("replace"),
        "expected 'replace' in message, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "CREATE OR REPLACE VIEW v AS SELECT 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn drop_view_no_violation() {
    let sql = "DROP VIEW v";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_or_replace_function_violation() {
    // sqlparser 0.53 supports CREATE OR REPLACE FUNCTION
    let sql = "CREATE OR REPLACE FUNCTION f() RETURNS INT LANGUAGE SQL AS $$ SELECT 1 $$";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn correct_col_for_create_keyword() {
    // CREATE starts at col 1
    let sql = "CREATE OR REPLACE VIEW v AS SELECT 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 1);
}
