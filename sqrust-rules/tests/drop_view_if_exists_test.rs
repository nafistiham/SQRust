use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::drop_view_if_exists::DropViewIfExists;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DropViewIfExists.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DropViewIfExists.name(), "Lint/DropViewIfExists");
}

#[test]
fn drop_view_without_if_exists_violation() {
    let sql = "DROP VIEW my_view";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn drop_view_if_exists_no_violation() {
    let sql = "DROP VIEW IF EXISTS my_view";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn drop_table_no_violation() {
    // Different object type — DropViewIfExists must not flag DROP TABLE
    let sql = "DROP TABLE t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn drop_view_violation_message_mentions_view_name() {
    let sql = "DROP VIEW my_view";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("my_view"),
        "message should mention the view name: {}",
        diags[0].message
    );
}

#[test]
fn drop_view_violation_message_mentions_if_exists() {
    let sql = "DROP VIEW my_view";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let lower = diags[0].message.to_lowercase();
    assert!(
        lower.contains("if exists"),
        "message should mention IF EXISTS: {}",
        diags[0].message
    );
}

#[test]
fn two_drop_views_two_violations() {
    let sql = "DROP VIEW view_a;\nDROP VIEW view_b";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn drop_view_if_exists_multiple_views_no_violation() {
    let sql = "DROP VIEW IF EXISTS view_a;\nDROP VIEW IF EXISTS view_b";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn drop_view_mixed() {
    // One with IF EXISTS (no violation), one without (1 violation)
    let sql = "DROP VIEW IF EXISTS safe_view;\nDROP VIEW risky_view";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_no_violations() {
    let sql = "NOT VALID SQL ###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = DropViewIfExists.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn line_col_nonzero() {
    let sql = "DROP VIEW my_view";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn drop_schema_no_violation() {
    // DROP SCHEMA is a different rule
    let sql = "DROP SCHEMA my_schema";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn drop_view_with_schema_prefix_violation() {
    let sql = "DROP VIEW schema.my_view";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
