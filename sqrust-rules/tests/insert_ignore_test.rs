use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::insert_ignore::InsertIgnore;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    InsertIgnore.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(InsertIgnore.name(), "Lint/InsertIgnore");
}

#[test]
fn insert_ignore_one_violation() {
    let sql = "INSERT IGNORE INTO t (a) VALUES (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_or_ignore_one_violation() {
    let sql = "INSERT OR IGNORE INTO t (a) VALUES (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_ignore_case_insensitive() {
    let sql = "insert ignore into t (a) values (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_or_ignore_case_insensitive() {
    let sql = "INSERT OR IGNORE into t (a) values (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn plain_insert_no_violation() {
    let sql = "INSERT INTO t (a) VALUES (1)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn insert_into_no_violation() {
    let sql = "INSERT INTO t SELECT a FROM src";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn insert_ignore_message_content() {
    let sql = "INSERT IGNORE INTO t (a) VALUES (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("mysql") || msg.contains("mysql-specific"),
        "message should mention MySQL: {}",
        diags[0].message
    );
}

#[test]
fn insert_or_ignore_message_content() {
    let sql = "INSERT OR IGNORE INTO t (a) VALUES (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("sqlite") || msg.contains("sqlite-specific"),
        "message should mention SQLite: {}",
        diags[0].message
    );
}

#[test]
fn two_insert_ignores_two_violations() {
    let sql = concat!(
        "INSERT IGNORE INTO t1 (a) VALUES (1);\n",
        "INSERT IGNORE INTO t2 (b) VALUES (2)",
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn insert_ignore_in_cte_context_violation() {
    let sql = concat!(
        "WITH cte AS (SELECT 1 AS id)\n",
        "INSERT IGNORE INTO t SELECT id FROM cte",
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn line_col_nonzero() {
    let sql = "INSERT IGNORE INTO t (a) VALUES (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn parse_error_ignored() {
    // Source-level scan always works regardless of parse errors
    let sql = "INSERT IGNORE INTO t (a) VALUES (### bad sql";
    let diags = check(sql);
    // Should still detect INSERT IGNORE even if SQL doesn't parse cleanly
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_ignore_column_value_violation() {
    let sql = "INSERT IGNORE INTO t (a, b) VALUES (1, 2)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_ignore_mixed_with_insert_or_ignore() {
    let sql = concat!(
        "INSERT IGNORE INTO t1 (a) VALUES (1);\n",
        "INSERT OR IGNORE INTO t2 (b) VALUES (2)",
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn select_no_violation() {
    let sql = "SELECT * FROM t WHERE a = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}
