use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::insert_or_replace::InsertOrReplace;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    InsertOrReplace.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(InsertOrReplace.name(), "Lint/InsertOrReplace");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "REPLACE INTO @@##GARBAGE";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = InsertOrReplace.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn replace_into_one_violation() {
    let sql = "REPLACE INTO t VALUES (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_or_replace_one_violation() {
    let sql = "INSERT OR REPLACE INTO t VALUES (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn regular_insert_no_violation() {
    let sql = "INSERT INTO t VALUES (1)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_replace_into_two_violations() {
    let sql = "REPLACE INTO t VALUES (1);\nREPLACE INTO u VALUES (2)";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn update_no_violation() {
    let sql = "UPDATE t SET col = 1 WHERE id = 2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_no_violation() {
    let sql = "SELECT * FROM t WHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_replace_or_conflict() {
    let sql = "REPLACE INTO t VALUES (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message.to_lowercase();
    assert!(
        msg.contains("replace") || msg.contains("conflict"),
        "message '{}' should mention 'replace' or 'conflict'",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "REPLACE INTO t VALUES (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn correct_line_for_replace_keyword() {
    let sql = "\nREPLACE INTO t VALUES (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn insert_ignore_no_violation() {
    // INSERT IGNORE is a different MySQL extension — we do NOT flag it.
    let sql = "INSERT IGNORE INTO t VALUES (1)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn col_nonzero() {
    let sql = "  REPLACE INTO t VALUES (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn insert_or_replace_correct_line() {
    let sql = "\nINSERT OR REPLACE INTO t VALUES (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}
