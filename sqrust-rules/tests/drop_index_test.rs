use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::drop_index::DropIndex;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DropIndex.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DropIndex.name(), "Lint/DropIndex");
}

#[test]
fn drop_index_without_if_exists_one_violation() {
    let sql = "DROP INDEX idx";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn drop_index_if_exists_no_violation() {
    let sql = "DROP INDEX IF EXISTS idx";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn drop_table_without_if_exists_no_violation() {
    // Different rule — DropIndex must not flag DROP TABLE
    let sql = "DROP TABLE t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn drop_table_if_exists_no_violation() {
    let sql = "DROP TABLE IF EXISTS t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiple_drop_index_multiple_violations() {
    let sql = "DROP INDEX idx1;\nDROP INDEX idx2";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "NOT VALID SQL ###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = DropIndex.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn drop_view_without_if_exists_no_violation() {
    // DropIndex must not flag DROP VIEW
    let sql = "DROP VIEW v";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_contains_if_exists() {
    let sql = "DROP INDEX idx";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("if exists"),
        "message should mention IF EXISTS: {}",
        diags[0].message
    );
}

#[test]
fn diagnostic_rule_name_correct() {
    let sql = "DROP INDEX idx";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Lint/DropIndex");
}

#[test]
fn if_exists_false_is_flagged() {
    // Explicit check: without IF EXISTS → violation
    let sql = "DROP INDEX my_idx";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn if_exists_true_is_not_flagged() {
    // Explicit check: with IF EXISTS → no violation
    let sql = "DROP INDEX IF EXISTS my_idx";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn line_col_nonzero() {
    let sql = "DROP INDEX idx";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn select_statement_no_violation() {
    let sql = "SELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}
