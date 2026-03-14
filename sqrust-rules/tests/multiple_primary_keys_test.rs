use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::multiple_primary_keys::MultiplePrimaryKeys;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    MultiplePrimaryKeys.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(MultiplePrimaryKeys.name(), "Lint/MultiplePrimaryKeys");
}

#[test]
fn two_column_primary_keys_violation() {
    let sql = "CREATE TABLE t (id INT PRIMARY KEY, code INT PRIMARY KEY)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn one_column_primary_key_no_violation() {
    let sql = "CREATE TABLE t (id INT PRIMARY KEY, name TEXT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn column_and_table_primary_key_violation() {
    let sql = "CREATE TABLE t (id INT PRIMARY KEY, CONSTRAINT pk PRIMARY KEY (id, code))";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_table_primary_key_constraints_violation() {
    let sql = "CREATE TABLE t (id INT, code INT, PRIMARY KEY (id), PRIMARY KEY (code))";
    // Two table-level PRIMARY KEY constraints — flag it.
    // Note: sqlparser may or may not accept this. If it fails to parse, no violations expected.
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        // Parser rejected it — acceptable
        return;
    }
    let diags = MultiplePrimaryKeys.check(&ctx);
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_primary_key_no_violation() {
    let sql = "CREATE TABLE t (id INT, name TEXT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_table_name() {
    let sql = "CREATE TABLE orders (id INT PRIMARY KEY, code INT PRIMARY KEY)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("orders"),
        "expected message to mention table name 'orders', got: {}",
        diags[0].message
    );
}

#[test]
fn message_mentions_count() {
    let sql = "CREATE TABLE t (id INT PRIMARY KEY, code INT PRIMARY KEY)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('2'),
        "expected message to mention count 2, got: {}",
        diags[0].message
    );
}

#[test]
fn parse_error_no_violations() {
    let sql = "CREATE INVALID @@## TABLE GARBAGE (";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = MultiplePrimaryKeys.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn line_col_nonzero() {
    let sql = "CREATE TABLE t (id INT PRIMARY KEY, code INT PRIMARY KEY)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn composite_primary_key_no_violation() {
    // Single table-level PK covering two columns — valid
    let sql = "CREATE TABLE t (id INT, code INT, PRIMARY KEY (id, code))";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn unique_constraint_no_violation() {
    // UNIQUE is not PRIMARY KEY
    let sql = "CREATE TABLE t (id INT UNIQUE, code INT UNIQUE)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_tables_one_each_no_violation() {
    // Each table has exactly one PK — no violation for either
    let sql = "CREATE TABLE a (id INT PRIMARY KEY);\nCREATE TABLE b (id INT PRIMARY KEY)";
    let diags = check(sql);
    assert!(diags.is_empty());
}
