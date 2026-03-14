use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::insert_overwrite::InsertOverwrite;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    InsertOverwrite.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(InsertOverwrite.name(), "Lint/InsertOverwrite");
}

#[test]
fn insert_overwrite_one_violation() {
    let sql = "INSERT OVERWRITE TABLE t SELECT * FROM src";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_overwrite_case_insensitive_upper() {
    let sql = "INSERT OVERWRITE TABLE t SELECT * FROM src";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_overwrite_case_insensitive_lower() {
    let sql = "insert overwrite table t select * from src";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_overwrite_mixed_case() {
    let sql = "Insert Overwrite Table t Select * From src";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn plain_insert_into_no_violation() {
    let sql = "INSERT INTO t (a) VALUES (1)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn insert_into_select_no_violation() {
    let sql = "INSERT INTO t SELECT a FROM src";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_no_violation() {
    let sql = "SELECT * FROM t WHERE a = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_hive_or_spark() {
    let sql = "INSERT OVERWRITE TABLE t SELECT * FROM src";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("hive") || msg.contains("spark"),
        "message should mention Hive or Spark: {}",
        diags[0].message
    );
}

#[test]
fn message_suggests_alternative() {
    let sql = "INSERT OVERWRITE TABLE t SELECT * FROM src";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("insert into") || msg.contains("create table"),
        "message should suggest an alternative: {}",
        diags[0].message
    );
}

#[test]
fn two_insert_overwrites_two_violations() {
    let sql = concat!(
        "INSERT OVERWRITE TABLE t1 SELECT * FROM src1;\n",
        "INSERT OVERWRITE TABLE t2 SELECT * FROM src2",
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn line_col_nonzero() {
    let sql = "INSERT OVERWRITE TABLE t SELECT * FROM src";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn insert_overwrite_on_second_line() {
    let sql = "-- setup\nINSERT OVERWRITE TABLE t SELECT * FROM src";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn insert_overwrite_in_string_literal_no_violation() {
    let sql = "SELECT 'INSERT OVERWRITE TABLE t SELECT * FROM src' AS q";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn insert_overwrite_in_comment_no_violation() {
    let sql = "-- INSERT OVERWRITE TABLE t SELECT * FROM src\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn parse_error_still_detects_violation() {
    // Source-level scan works regardless of parse errors
    let sql = "INSERT OVERWRITE TABLE t SELECT * FROM (### bad";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
