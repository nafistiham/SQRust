use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::delete_without_where::DeleteWithoutWhere;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DeleteWithoutWhere.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DeleteWithoutWhere.name(), "Lint/DeleteWithoutWhere");
}

#[test]
fn delete_without_where_one_violation() {
    let sql = "DELETE FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn delete_with_where_no_violation() {
    let sql = "DELETE FROM t WHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn delete_lowercase_without_where_one_violation() {
    let sql = "delete from t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn delete_with_complex_where_no_violation() {
    let sql = "DELETE FROM t WHERE id > 100";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiple_deletes_only_one_without_where_flagged() {
    let sql = "DELETE FROM t WHERE id = 1;\nDELETE FROM u";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_deletes_both_without_where_two_violations() {
    let sql = "DELETE FROM t;\nDELETE FROM u";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn delete_with_tautological_where_no_violation() {
    // WHERE 1=1 is still a WHERE clause — do not flag it
    let sql = "DELETE FROM t WHERE 1=1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn mixed_select_and_delete_without_where_one_violation() {
    let sql = "SELECT * FROM t;\nDELETE FROM u";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_empty() {
    let sql = "DELETEE INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = DeleteWithoutWhere.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn message_format_is_correct() {
    let sql = "DELETE FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "DELETE without WHERE clause will delete all rows"
    );
}

#[test]
fn delete_with_false_condition_where_no_violation() {
    // WHERE active = false is still a WHERE clause — do not flag it
    let sql = "DELETE FROM t WHERE active = false";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn correct_line_number_for_delete_keyword() {
    // DELETE is on line 2
    let sql = "SELECT 1;\nDELETE FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn correct_col_number_for_delete_keyword() {
    // DELETE starts at column 1 on a fresh line
    let sql = "DELETE FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 1);
}
