use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::insert_without_column_list::InsertWithoutColumnList;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    InsertWithoutColumnList.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        InsertWithoutColumnList.name(),
        "Lint/InsertWithoutColumnList"
    );
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "INSERTTT GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = InsertWithoutColumnList.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn insert_without_columns_one_violation() {
    let sql = "INSERT INTO t VALUES (1, 2)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_with_columns_no_violation() {
    let sql = "INSERT INTO t (a, b) VALUES (1, 2)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn insert_with_single_column_and_select_no_violation() {
    let sql = "INSERT INTO t (a) SELECT a FROM s";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn insert_without_columns_using_select_one_violation() {
    let sql = "INSERT INTO t SELECT a FROM s";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_without_columns_multiple_rows_one_violation() {
    let sql = "INSERT INTO t VALUES (1), (2)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_inserts_one_with_columns_one_without_one_violation() {
    let sql = "INSERT INTO t (a, b) VALUES (1, 2);\nINSERT INTO u VALUES (3, 4)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_query_no_violation() {
    let sql = "SELECT * FROM t WHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn no_insert_no_violation() {
    let sql = "UPDATE t SET col = 1 WHERE id = 2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn line_col_is_non_zero() {
    let sql = "INSERT INTO t VALUES (1, 2)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn message_format_is_correct() {
    let sql = "INSERT INTO t VALUES (1, 2)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "INSERT statement missing explicit column list; specify columns for safety"
    );
}

#[test]
fn qualified_table_name_without_columns_one_violation() {
    let sql = "INSERT INTO schema.table VALUES (1)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_keyword_on_first_line_col_one() {
    let sql = "INSERT INTO t VALUES (1, 2)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 1);
}
