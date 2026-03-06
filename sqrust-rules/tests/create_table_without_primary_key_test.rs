use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::create_table_without_primary_key::CreateTableWithoutPrimaryKey;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CreateTableWithoutPrimaryKey.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        CreateTableWithoutPrimaryKey.name(),
        "Lint/CreateTableWithoutPrimaryKey"
    );
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "CREATE INVALID @@## TABLE GARBAGE (";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = CreateTableWithoutPrimaryKey.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn create_table_without_pk_one_violation() {
    let sql = "CREATE TABLE t (id INT, name TEXT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_table_with_column_pk_no_violation() {
    let sql = "CREATE TABLE t (id INT PRIMARY KEY, name TEXT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_table_with_table_pk_no_violation() {
    let sql = "CREATE TABLE t (id INT, name TEXT, PRIMARY KEY (id))";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_table_with_composite_pk_no_violation() {
    let sql = "CREATE TABLE t (a INT, b INT, PRIMARY KEY (a, b))";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_table_if_not_exists_without_pk_violation() {
    let sql = "CREATE TABLE IF NOT EXISTS t (id INT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_no_violation() {
    let sql = "SELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn insert_no_violation() {
    let sql = "INSERT INTO t VALUES (1)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_create_without_pk_two_violations() {
    let sql = "CREATE TABLE a (x INT);\nCREATE TABLE b (y TEXT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn create_table_with_unique_still_flags_no_pk() {
    // UNIQUE is not a PRIMARY KEY — the table still has no PK.
    let sql = "CREATE TABLE t (id INT UNIQUE, name TEXT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_useful_text() {
    let sql = "CREATE TABLE t (id INT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("PRIMARY KEY") || diags[0].message.contains("primary key"),
        "expected message to mention PRIMARY KEY, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "CREATE TABLE t (id INT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn create_table_empty_no_violation_or_violation_handled_gracefully() {
    // CREATE TABLE t () — no columns at all. Whether this is flagged or not,
    // it must not panic.
    let sql = "CREATE TABLE t ()";
    let ctx = FileContext::from_source(sql, "test.sql");
    // Just ensure it runs without panic. If parsed: 0 columns → no PK → 1 violation.
    let diags = CreateTableWithoutPrimaryKey.check(&ctx);
    // We assert it does not panic (implicitly verified by reaching here).
    // If parsed successfully, we expect a violation.
    if ctx.parse_errors.is_empty() {
        assert_eq!(diags.len(), 1);
    }
}

#[test]
fn create_table_with_constraint_pk_no_violation() {
    // CONSTRAINT pk_name PRIMARY KEY (id) — table-level with explicit constraint name.
    let sql = "CREATE TABLE t (id INT, CONSTRAINT pk_t PRIMARY KEY (id))";
    let diags = check(sql);
    assert!(diags.is_empty());
}
