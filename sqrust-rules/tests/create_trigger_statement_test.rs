use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::create_trigger_statement::CreateTriggerStatement;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CreateTriggerStatement.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        CreateTriggerStatement.name(),
        "Lint/CreateTriggerStatement"
    );
}

#[test]
fn basic_create_trigger_violation() {
    let sql = "CREATE TRIGGER trg_audit\nBEFORE INSERT ON orders\nFOR EACH ROW BEGIN\n  INSERT INTO audit_log VALUES (NEW.id);\nEND;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn single_line_query_no_violation() {
    let sql = "SELECT * FROM orders WHERE status = 'active'";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_trigger_in_string_no_violation() {
    let sql = "SELECT 'CREATE TRIGGER foo' AS example FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_trigger_in_comment_no_violation() {
    let sql = "-- CREATE TRIGGER trg_audit\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_trigger_in_block_comment_no_violation() {
    let sql = "/* CREATE TRIGGER trg_audit\nBEFORE INSERT ON orders */\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_trigger_before_on_keyword() {
    let sql = "CREATE TRIGGER trg_check BEFORE INSERT ON my_table FOR EACH ROW BEGIN END;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn create_trigger_after_drop() {
    let sql = "DROP TRIGGER IF EXISTS old_trg;\nCREATE TRIGGER new_trg AFTER UPDATE ON t FOR EACH ROW BEGIN END;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn empty_file_no_violation() {
    let sql = "";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn uppercase_violation() {
    let sql = "CREATE TRIGGER trg_upper AFTER DELETE ON t FOR EACH ROW BEGIN END;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_violation() {
    let sql = "create trigger trg_lower after delete on t for each row begin end;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_case_violation() {
    let sql = "Create Trigger trg_mixed After Delete On t For Each Row Begin End;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_table_no_violation() {
    let sql = "CREATE TABLE orders (id INT PRIMARY KEY, status TEXT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_trigger() {
    let sql = "CREATE TRIGGER trg_test BEFORE INSERT ON t FOR EACH ROW BEGIN END;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("trigger"),
        "message should mention trigger, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "CREATE TRIGGER trg_test BEFORE INSERT ON t FOR EACH ROW BEGIN END;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
