use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::duplicate_column_in_create::DuplicateColumnInCreate;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DuplicateColumnInCreate.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DuplicateColumnInCreate.name(), "Lint/DuplicateColumnInCreate");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = DuplicateColumnInCreate.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn no_duplicate_columns_no_violation() {
    let sql = "CREATE TABLE t (a INT, b TEXT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn duplicate_column_one_violation() {
    let sql = "CREATE TABLE t (a INT, a TEXT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains('a'));
}

#[test]
fn duplicate_column_three_columns_one_violation() {
    let sql = "CREATE TABLE t (a INT, b TEXT, a VARCHAR(10))";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn duplicate_column_case_insensitive_one_violation() {
    let sql = "CREATE TABLE t (A INT, a TEXT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_distinct_columns_no_violation() {
    let sql = "CREATE TABLE t (a INT, b INT, c INT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_query_no_violation() {
    let sql = "SELECT a, b FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn no_create_table_no_violation() {
    let sql = "INSERT INTO t (a, b) VALUES (1, 2)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiple_duplicate_pairs_multiple_violations() {
    // Both 'a' and 'b' appear twice — should get 2 violations.
    let sql = "CREATE TABLE t (a INT, b INT, a TEXT, b TEXT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_contains_column_name() {
    let sql = "CREATE TABLE t (my_col INT, my_col TEXT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("my_col"));
}

#[test]
fn line_col_non_zero() {
    let sql = "CREATE TABLE t (a INT, a TEXT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn three_columns_all_same_name_one_violation() {
    // All three are 'a' — should be reported once.
    let sql = "CREATE TABLE t (a INT, a TEXT, a VARCHAR(10))";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn qualified_table_name_duplicate_one_violation() {
    let sql = "CREATE TABLE schema_name.t (a INT, a TEXT)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
