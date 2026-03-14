use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::merge_statement::MergeStatement;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    MergeStatement.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(MergeStatement.name(), "Lint/MergeStatement");
}

#[test]
fn rule_name_starts_with_lint_prefix() {
    assert!(MergeStatement.name().starts_with("Lint/"));
}

#[test]
fn basic_merge_one_violation() {
    let sql = "MERGE INTO target t \
               USING source s ON t.id = s.id \
               WHEN MATCHED THEN UPDATE SET t.val = s.val \
               WHEN NOT MATCHED THEN INSERT (id, val) VALUES (s.id, s.val)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_no_violation() {
    let sql = "INSERT INTO t (a, b) VALUES (1, 2)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn update_no_violation() {
    let sql = "UPDATE t SET a = 1 WHERE id = 2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn delete_no_violation() {
    let sql = "DELETE FROM t WHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_table_no_violation() {
    let sql = "CREATE TABLE t (id INT, val TEXT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_no_violation() {
    let sql = "SELECT * FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "NOT VALID SQL ###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = MergeStatement.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn multiple_merges_multiple_violations() {
    let sql = concat!(
        "MERGE INTO t1 USING s1 ON t1.id = s1.id WHEN MATCHED THEN DELETE;\n",
        "MERGE INTO t2 USING s2 ON t2.id = s2.id WHEN MATCHED THEN DELETE",
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_mentions_not_supported_or_compatible() {
    let sql = "MERGE INTO target t \
               USING source s ON t.id = s.id \
               WHEN MATCHED THEN UPDATE SET t.val = s.val \
               WHEN NOT MATCHED THEN INSERT (id, val) VALUES (s.id, s.val)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("not supported") || msg.contains("compatible") || msg.contains("support"),
        "message should mention compatibility: {}",
        diags[0].message
    );
}

#[test]
fn diagnostic_rule_name_matches() {
    let sql = "MERGE INTO target t \
               USING source s ON t.id = s.id \
               WHEN MATCHED THEN UPDATE SET t.val = s.val \
               WHEN NOT MATCHED THEN INSERT (id, val) VALUES (s.id, s.val)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Lint/MergeStatement");
}

#[test]
fn line_col_nonzero() {
    let sql = "MERGE INTO target t \
               USING source s ON t.id = s.id \
               WHEN MATCHED THEN UPDATE SET t.val = s.val \
               WHEN NOT MATCHED THEN INSERT (id, val) VALUES (s.id, s.val)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn correct_line_for_merge_keyword() {
    let sql = concat!(
        "SELECT 1;\n",
        "MERGE INTO target t USING source s ON t.id = s.id WHEN MATCHED THEN DELETE",
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn merge_into_without_into_keyword() {
    // MERGE without INTO is also valid in some dialects
    let sql = "MERGE target t \
               USING source s ON t.id = s.id \
               WHEN MATCHED THEN UPDATE SET t.val = s.val \
               WHEN NOT MATCHED THEN INSERT (id, val) VALUES (s.id, s.val)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn merge_in_multi_statement_file() {
    let sql = concat!(
        "SELECT 1;\n",
        "MERGE INTO target t USING source s ON t.id = s.id WHEN MATCHED THEN DELETE;\n",
        "SELECT 2",
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
