use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::cross_database_reference::CrossDatabaseReference;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CrossDatabaseReference.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CrossDatabaseReference.name(), "Lint/CrossDatabaseReference");
}

#[test]
fn three_part_name_select_from_one_violation() {
    let sql = "SELECT * FROM db.schema.tbl";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_part_name_no_violation() {
    let sql = "SELECT * FROM schema.tbl";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn one_part_name_no_violation() {
    let sql = "SELECT * FROM tbl";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn four_part_name_one_violation() {
    let sql = "SELECT * FROM catalog.db.schema.tbl";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_part_name_in_join_one_violation() {
    let sql = "SELECT * FROM a JOIN db.schema.other ON a.id = other.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn simple_inner_join_no_violation() {
    let sql = "SELECT * FROM a INNER JOIN b ON a.id = b.id";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiple_three_part_refs_multiple_violations() {
    let sql = "SELECT * FROM db.schema.t1 JOIN db.schema.t2 ON t1.id = t2.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn two_part_refs_in_join_no_violation() {
    let sql = "SELECT * FROM schema.t1 JOIN schema.t2 ON t1.id = t2.id";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn insert_into_three_part_one_violation() {
    let sql = "INSERT INTO db.schema.tbl SELECT * FROM x";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn update_three_part_one_violation() {
    let sql = "UPDATE db.schema.tbl SET col = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECT @@@ FROM @@@";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = CrossDatabaseReference.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn delete_from_three_part_one_violation() {
    let sql = "DELETE FROM db.schema.tbl WHERE id = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_three_part_name() {
    let sql = "SELECT * FROM mydb.myschema.mytable";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("mydb.myschema.mytable"),
        "expected message to contain 'mydb.myschema.mytable', got: {}",
        msg
    );
}
