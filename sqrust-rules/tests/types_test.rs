use sqrust_core::FileContext;
use sqrust_rules::capitalisation::types::Types;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    Types.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(Types.name(), "Capitalisation/Types");
}

#[test]
fn uppercase_int_no_violation() {
    assert!(check("CREATE TABLE t (id INT)").is_empty());
}

#[test]
fn lowercase_int_flagged() {
    let diags = check("CREATE TABLE t (id int)");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Capitalisation/Types");
    assert_eq!(diags[0].message, "Data type 'int' should be 'INT'");
}

#[test]
fn mixed_case_integer_flagged() {
    let diags = check("CREATE TABLE t (id Integer)");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "Data type 'Integer' should be 'INTEGER'");
}

#[test]
fn lowercase_varchar_flagged() {
    let diags = check("CREATE TABLE t (name varchar(255))");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "Data type 'varchar' should be 'VARCHAR'");
}

#[test]
fn multiple_correct_types_no_violation() {
    assert!(check("CREATE TABLE t (id INT, name VARCHAR(100))").is_empty());
}

#[test]
fn type_inside_string_no_violation() {
    assert!(check("SELECT 'int'").is_empty());
}

#[test]
fn type_inside_line_comment_no_violation() {
    assert!(check("SELECT 1 -- int type here").is_empty());
}

#[test]
fn type_inside_block_comment_no_violation() {
    assert!(check("SELECT 1 /* int type here */").is_empty());
}

#[test]
fn cast_with_lowercase_integer_flagged() {
    let diags = check("SELECT CAST(x AS integer) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "Data type 'integer' should be 'INTEGER'");
}

#[test]
fn cast_with_uppercase_integer_no_violation() {
    assert!(check("SELECT CAST(x AS INTEGER) FROM t").is_empty());
}

#[test]
fn correct_line_number_for_violation_on_line_3() {
    let sql = "SELECT 1;\nSELECT 2;\nCREATE TABLE t (id integer)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}

#[test]
fn correct_col_number_for_violation() {
    // "CREATE TABLE t (id integer)"
    // C(1)...(16)i(17)d(18) (19)i(20)n(21)t(22)e(23)g(24)e(25)r(26)
    // "integer" starts at col 20
    let diags = check("CREATE TABLE t (id integer)");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 20);
}

#[test]
fn lowercase_timestamp_flagged_with_correct_message() {
    let diags = check("CREATE TABLE t (created_at timestamp)");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "Data type 'timestamp' should be 'TIMESTAMP'");
}

#[test]
fn word_containing_type_name_not_flagged() {
    // "integer_id" contains "integer" but is a longer word — must not flag
    assert!(check("SELECT integer_id FROM t").is_empty());
}

#[test]
fn bigint_matched_not_int() {
    // "bigint" should produce a violation for 'bigint' -> 'BIGINT', not 'int' -> 'INT'
    let diags = check("CREATE TABLE t (id bigint)");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "Data type 'bigint' should be 'BIGINT'");
}
