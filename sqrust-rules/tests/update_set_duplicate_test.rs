use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::update_set_duplicate::UpdateSetDuplicate;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    UpdateSetDuplicate.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UpdateSetDuplicate.name(), "Lint/UpdateSetDuplicate");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "UPDATEE INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = UpdateSetDuplicate.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn distinct_columns_no_violation() {
    let sql = "UPDATE t SET a = 1, b = 2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn duplicate_column_one_violation() {
    let sql = "UPDATE t SET a = 1, a = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn duplicate_column_message_contains_column_name() {
    let sql = "UPDATE t SET a = 1, a = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("a"));
}

#[test]
fn three_assignments_first_and_third_same_one_violation() {
    let sql = "UPDATE t SET a = 1, b = 2, a = 3";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_different_duplicate_columns_two_violations() {
    let sql = "UPDATE t SET a = 1, b = 2, a = 3, b = 4";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn single_assignment_no_violation() {
    let sql = "UPDATE t SET a = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn duplicate_column_case_insensitive_one_violation() {
    let sql = "UPDATE t SET A = 1, a = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_update_no_violation() {
    let sql = "DELETE FROM t WHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_query_no_violation() {
    let sql = "SELECT a, a FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_format_is_correct() {
    let sql = "UPDATE t SET col = 1, col = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "Column 'col' appears more than once in UPDATE SET clause"
    );
}

#[test]
fn line_and_col_are_nonzero() {
    let sql = "UPDATE t SET a = 1, a = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn update_with_where_and_duplicate_set_one_violation() {
    let sql = "UPDATE t SET a = 1, a = 2 WHERE id = 5";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_update_statements_each_with_duplicate_two_violations() {
    let sql = "UPDATE t SET a = 1, a = 2;\nUPDATE u SET b = 1, b = 2";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}
