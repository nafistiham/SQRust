use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::select_into_table::SelectIntoTable;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SelectIntoTable.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelectIntoTable.name(), "Lint/SelectIntoTable");
}

#[test]
fn select_into_new_table_one_violation() {
    let sql = "SELECT a, b INTO new_table FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn plain_select_no_violation() {
    let sql = "SELECT a FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_into_temp_table_one_violation() {
    let sql = "SELECT a, b INTO #temp FROM t";
    let ctx = FileContext::from_source(sql, "test.sql");
    // If parsed successfully, must produce exactly 1 violation.
    if ctx.parse_errors.is_empty() {
        let diags = SelectIntoTable.check(&ctx);
        assert_eq!(diags.len(), 1);
    }
    // If dialect rejects #temp, parse fails and we accept 0 violations.
}

#[test]
fn insert_into_select_no_violation() {
    let sql = "INSERT INTO t SELECT a FROM s";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_star_into_backup_with_where_one_violation() {
    let sql = "SELECT * INTO backup_table FROM t WHERE id > 100";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECT INTO @@### GARBAGE";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = SelectIntoTable.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn select_into_qualified_table_one_violation() {
    let sql = "SELECT a, b, c INTO schema.new_table FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_select_into_multiple_violations() {
    let sql = "SELECT a INTO t1 FROM s1;\nSELECT b INTO t2 FROM s2";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn create_table_as_select_no_violation() {
    let sql = "CREATE TABLE t AS SELECT a FROM s";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_contains_select_into_or_create_table() {
    let sql = "SELECT a INTO new_table FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("select into") || msg.contains("create table"),
        "expected message to mention 'SELECT INTO' or 'CREATE TABLE', got: {}",
        diags[0].message
    );
}

#[test]
fn diagnostic_rule_name_is_correct() {
    let sql = "SELECT a INTO new_table FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Lint/SelectIntoTable");
}

#[test]
fn line_col_nonzero() {
    let sql = "SELECT a INTO new_table FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn cte_with_select_into_if_parseable_one_violation() {
    // CTE body with SELECT INTO — unusual but check parse-guard behaviour.
    let sql = "WITH c AS (SELECT a INTO x FROM t) SELECT * FROM c";
    let ctx = FileContext::from_source(sql, "test.sql");
    if ctx.parse_errors.is_empty() {
        // If sqlparser accepts this, we must flag it.
        let diags = SelectIntoTable.check(&ctx);
        assert_eq!(diags.len(), 1);
    }
    // If it fails to parse, 0 violations is correct.
}

#[test]
fn update_statement_no_violation() {
    let sql = "UPDATE t SET a = 1 WHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}
