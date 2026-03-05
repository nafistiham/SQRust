use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::update_without_where::UpdateWithoutWhere;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    UpdateWithoutWhere.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UpdateWithoutWhere.name(), "Lint/UpdateWithoutWhere");
}

#[test]
fn update_without_where_one_violation() {
    let sql = "UPDATE t SET col = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn update_with_where_no_violation() {
    let sql = "UPDATE t SET col = 1 WHERE id = 5";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn update_lowercase_without_where_one_violation() {
    let sql = "update t set col = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_updates_only_one_without_where_flagged() {
    let sql = "UPDATE t SET col = 1 WHERE id = 5;\nUPDATE u SET col = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_updates_both_without_where_two_violations() {
    let sql = "UPDATE t SET col = 1;\nUPDATE u SET col = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn update_with_tautological_where_no_violation() {
    // WHERE 1=1 is still a WHERE clause — do not flag it
    let sql = "UPDATE t SET col = 1 WHERE 1=1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn mixed_select_and_update_without_where_one_violation() {
    let sql = "SELECT * FROM t;\nUPDATE u SET col = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_empty() {
    let sql = "UPDATEE INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = UpdateWithoutWhere.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn message_format_is_correct() {
    let sql = "UPDATE t SET col = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "UPDATE without WHERE clause will update all rows"
    );
}

#[test]
fn update_with_complex_where_no_violation() {
    let sql = "UPDATE t SET active = true WHERE status = 'pending'";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn update_multiple_assignments_without_where_one_violation() {
    // Multiple SET assignments, no WHERE — still 1 violation
    let sql = "UPDATE t SET col1 = 1, col2 = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn correct_line_number_for_update_keyword() {
    // UPDATE is on line 2
    let sql = "SELECT 1;\nUPDATE t SET col = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn correct_col_number_for_update_keyword() {
    // UPDATE starts at column 1 on a fresh line
    let sql = "UPDATE t SET col = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 1);
}
