use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::consistent_quote_style::ConsistentQuoteStyle;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ConsistentQuoteStyle.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ConsistentQuoteStyle.name(), "Layout/ConsistentQuoteStyle");
}

#[test]
fn only_single_quotes_no_violations() {
    let diags = check("SELECT 'hello' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn only_double_quotes_no_violations() {
    let diags = check("SELECT \"hello\" FROM t");
    assert!(diags.is_empty());
}

#[test]
fn mixed_single_and_double_quotes_one_violation() {
    let diags = check("SELECT 'hello', \"world\" FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_single_quotes_no_violations() {
    let diags = check("SELECT 'a', 'b', 'c' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn multiple_double_quotes_no_violations() {
    let diags = check("SELECT \"a\", \"b\" FROM t");
    assert!(diags.is_empty());
}

#[test]
fn mixed_in_where_clause_one_violation() {
    let diags = check("SELECT * FROM t WHERE a = 'foo' AND b = \"bar\"");
    assert_eq!(diags.len(), 1);
}

#[test]
fn double_quote_in_line_comment_no_violations() {
    let diags = check("SELECT * FROM t WHERE a = 'foo' -- and b = \"bar\"");
    assert!(diags.is_empty());
}

#[test]
fn double_quote_in_block_comment_no_violations() {
    let diags = check("SELECT * FROM t WHERE a = 'foo' /* and b = \"bar\" */");
    assert!(diags.is_empty());
}

#[test]
fn empty_sql_no_violations() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn only_whitespace_no_violations() {
    let diags = check("   \n  \t  ");
    assert!(diags.is_empty());
}

#[test]
fn escaped_single_quote_inside_string_no_violations() {
    let diags = check("SELECT 'it''s a test' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn mixed_with_multiple_singles_one_violation() {
    let diags = check("SELECT 'foo', 'bar', \"baz\" FROM t WHERE x = 'qux'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiline_with_mixed_styles_one_violation() {
    let sql = "SELECT\n    'foo',\n    \"bar\"\nFROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn violation_message_is_correct() {
    let diags = check("SELECT 'hello', \"world\" FROM t");
    assert_eq!(
        diags[0].message,
        "Mixed string quote styles detected — use single quotes consistently for string literals"
    );
}

#[test]
fn violation_rule_name_is_correct() {
    let diags = check("SELECT 'hello', \"world\" FROM t");
    assert_eq!(diags[0].rule, "Layout/ConsistentQuoteStyle");
}
