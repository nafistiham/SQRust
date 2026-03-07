use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::grant_all_privileges::GrantAllPrivileges;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    GrantAllPrivileges.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(GrantAllPrivileges.name(), "Lint/GrantAllPrivileges");
}

#[test]
fn grant_specific_no_violation() {
    let sql = "GRANT SELECT ON t TO user1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn grant_all_one_violation() {
    let sql = "GRANT ALL ON t TO user1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn grant_all_privileges_one_violation() {
    let sql = "GRANT ALL PRIVILEGES ON t TO user1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn grant_all_case_insensitive() {
    let sql = "grant all on t to user1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn grant_all_in_string_not_flagged() {
    let sql = "SELECT 'GRANT ALL' FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn grant_all_in_comment_not_flagged() {
    let sql = "-- GRANT ALL ON t TO user1\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_grant_all_two_violations() {
    let sql = "GRANT ALL ON t TO user1;\nGRANT ALL ON u TO user2";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn grant_select_insert_no_violation() {
    let sql = "GRANT SELECT, INSERT ON t TO user1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_permissive() {
    let sql = "GRANT ALL ON t TO user1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("permissive"),
        "message should mention 'permissive', got: {}",
        diags[0].message
    );
}

#[test]
fn line_nonzero() {
    let sql = "GRANT ALL ON t TO user1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

#[test]
fn col_nonzero() {
    let sql = "GRANT ALL ON t TO user1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn grant_all_col_points_to_grant() {
    // GRANT starts at column 1 on line 1
    let sql = "GRANT ALL ON t TO user1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn grant_all_without_on_flagged() {
    // GRANT ALL TO user1 — no ON clause, still flagged
    let sql = "GRANT ALL TO user1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
