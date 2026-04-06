use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::string_literal_newline::StringLiteralNewline;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    StringLiteralNewline.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(StringLiteralNewline.name(), "Ambiguous/StringLiteralNewline");
}

#[test]
fn newline_in_string_violation() {
    // The \n in the Rust string is an actual newline byte (0x0A).
    let sql = "SELECT 'hello\nworld' FROM t";
    let d = check(sql);
    assert_eq!(d.len(), 1);
}

#[test]
fn normal_string_no_violation() {
    let d = check("SELECT 'hello world' FROM t");
    assert_eq!(d.len(), 0);
}

#[test]
fn escaped_n_no_violation() {
    // \\n in a Rust string literal is a literal backslash followed by 'n',
    // which in SQL represents two characters — not a newline.
    let sql = "SELECT 'hello\\nworld' FROM t";
    let d = check(sql);
    assert_eq!(d.len(), 0);
}

#[test]
fn empty_string_no_violation() {
    let d = check("SELECT '' FROM t");
    assert_eq!(d.len(), 0);
}

#[test]
fn multiline_sql_without_string_newline_no_violation() {
    // The SQL itself spans multiple lines, but the strings do not.
    let sql = "SELECT\n  'hello',\n  'world'\nFROM t";
    let d = check(sql);
    assert_eq!(d.len(), 0);
}

#[test]
fn newline_at_start_of_string_violation() {
    let sql = "SELECT '\nhello' FROM t";
    let d = check(sql);
    assert_eq!(d.len(), 1);
}

#[test]
fn newline_at_end_of_string_violation() {
    let sql = "SELECT 'hello\n' FROM t";
    let d = check(sql);
    assert_eq!(d.len(), 1);
}

#[test]
fn multiple_strings_one_with_newline_one_violation() {
    // Two string literals, only the second contains a newline.
    let sql = "SELECT 'good', 'bad\none' FROM t";
    let d = check(sql);
    assert_eq!(d.len(), 1);
}

#[test]
fn multiple_strings_both_with_newlines_two_violations() {
    let sql = "SELECT 'a\nb', 'c\nd' FROM t";
    let d = check(sql);
    assert_eq!(d.len(), 2);
}

#[test]
fn string_in_comment_no_violation() {
    // A line comment where the content after -- does NOT contain a quote pair
    // that would trick the scanner. The comment ends at the newline; the
    // actual SQL on the next line has no string with a newline.
    let sql = "-- this is a comment with no string\nSELECT 'hello' FROM t";
    let d = check(sql);
    assert_eq!(d.len(), 0);
}

#[test]
fn empty_file_no_violation() {
    let d = check("");
    assert_eq!(d.len(), 0);
}

#[test]
fn violation_at_correct_line() {
    // The string with the newline starts on line 2.
    let sql = "SELECT 1,\n'hello\nworld' FROM t";
    let d = check(sql);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].line, 2, "expected violation on line 2, got line {}", d[0].line);
}
