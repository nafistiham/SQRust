use sqrust_core::FileContext;
use sqrust_rules::convention::no_char_type::NoCharType;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoCharType.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoCharType.name(), "Convention/NoCharType");
}

#[test]
fn parse_error_produces_no_violations() {
    // Trigger a parse error — rule must return empty
    let diags = check("SELECT FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn char_with_length_flagged() {
    // CREATE TABLE t (col CHAR(10))
    // "CHAR" starts at col 21
    let diags = check("CREATE TABLE t (col CHAR(10))");
    assert_eq!(diags.len(), 1);
}

#[test]
fn varchar_not_flagged() {
    // VARCHAR contains "CHAR" but must not be flagged
    let diags = check("CREATE TABLE t (col VARCHAR(10))");
    assert!(diags.is_empty());
}

#[test]
fn bare_char_flagged() {
    // CHAR without a length parameter
    let diags = check("CREATE TABLE t (col CHAR)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nchar_not_flagged() {
    // NCHAR is a separate type — CHAR is preceded by 'N', a word char
    let diags = check("CREATE TABLE t (col NCHAR(10))");
    assert!(diags.is_empty());
}

#[test]
fn char_in_line_comment_not_flagged() {
    // CHAR inside -- ... should be skipped
    let diags = check("SELECT 1 -- use CHAR type here");
    assert!(diags.is_empty());
}

#[test]
fn char_in_string_literal_not_flagged() {
    // CHAR inside single-quoted string must be skipped
    let diags = check("SELECT 'CHAR type is padded' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn char_in_block_comment_not_flagged() {
    // CHAR inside /* ... */ must be skipped
    let diags = check("SELECT 1 /* CHAR is bad */ FROM t");
    assert!(diags.is_empty());
}

#[test]
fn lowercase_char_flagged() {
    // char(10) — case-insensitive match
    let diags = check("CREATE TABLE t (col char(10))");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_char_columns_multiple_violations() {
    let sql = "CREATE TABLE t (a CHAR(5), b CHAR(10))";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn line_col_points_to_char_keyword() {
    // "CREATE TABLE t (col CHAR(10))"
    //  123456789012345678901234
    // 'C' of CHAR is at col 21
    let diags = check("CREATE TABLE t (col CHAR(10))");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 21);
}

#[test]
fn message_format_correct() {
    let diags = check("CREATE TABLE t (col CHAR(10))");
    assert_eq!(
        diags[0].message,
        "CHAR type used; prefer VARCHAR for variable-length strings"
    );
}

#[test]
fn charvar_not_flagged() {
    // CHARVAR — CHAR followed by word char V, must NOT be flagged
    let diags = check("SELECT charvar FROM t");
    assert!(diags.is_empty());
}

#[test]
fn char_at_end_of_line_flagged() {
    // CHAR at end of file / line with no trailing character
    let sql = "CREATE TABLE t (\n  col CHAR\n)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}
