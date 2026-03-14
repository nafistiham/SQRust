use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::select_for_update::SelectForUpdate;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SelectForUpdate.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelectForUpdate.name(), "Lint/SelectForUpdate");
}

#[test]
fn for_update_violation() {
    let sql = "SELECT * FROM t FOR UPDATE";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn for_share_violation() {
    let sql = "SELECT * FROM t FOR SHARE";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn with_updlock_violation() {
    let sql = "SELECT * FROM t WITH (UPDLOCK)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn for_update_case_insensitive() {
    let sql = "select * from t for update";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_locking_no_violation() {
    let sql = "SELECT * FROM t WHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn for_update_in_string_no_violation() {
    let sql = "SELECT 'FOR UPDATE example' FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn for_update_in_comment_no_violation() {
    let sql = "-- FOR UPDATE\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiple_for_update_multiple_violations() {
    let sql = "SELECT * FROM t FOR UPDATE;\nSELECT * FROM u FOR UPDATE";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn for_update_message_content() {
    let sql = "SELECT * FROM t FOR UPDATE";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let lower = diags[0].message.to_lowercase();
    assert!(
        lower.contains("for update"),
        "message should mention FOR UPDATE: {}",
        diags[0].message
    );
}

#[test]
fn for_share_message_content() {
    let sql = "SELECT * FROM t FOR SHARE";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let lower = diags[0].message.to_lowercase();
    assert!(
        lower.contains("for share"),
        "message should mention FOR SHARE: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "SELECT * FROM t FOR UPDATE";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn for_no_key_update_violation() {
    let sql = "SELECT * FROM t FOR NO KEY UPDATE";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn for_update_of_violation() {
    let sql = "SELECT * FROM t FOR UPDATE OF t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
