use sqrust_core::FileContext;
use sqrust_rules::ambiguous::order_by_position::OrderByPosition;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    OrderByPosition.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(OrderByPosition.name(), "Ambiguous/OrderByPosition");
}

#[test]
fn column_name_no_violation() {
    assert!(check("SELECT a FROM t ORDER BY a").is_empty());
}

#[test]
fn single_positional_ref_flagged() {
    let diags = check("SELECT a FROM t ORDER BY 1");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/OrderByPosition");
}

#[test]
fn positional_ref_with_desc_flagged() {
    let diags = check("SELECT a FROM t ORDER BY 1 DESC");
    assert_eq!(diags.len(), 1);
}

#[test]
fn positional_ref_with_asc_flagged() {
    let diags = check("SELECT a FROM t ORDER BY 1 ASC");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_positional_refs_flagged() {
    let diags = check("SELECT a, b FROM t ORDER BY 1, 2");
    assert_eq!(diags.len(), 2);
}

#[test]
fn mixed_positional_and_column_only_integer_flagged() {
    let diags = check("SELECT a, b FROM t ORDER BY a, 2");
    assert_eq!(diags.len(), 1);
}

#[test]
fn order_by_in_string_not_flagged() {
    assert!(check("SELECT 'ORDER BY 1' FROM t ORDER BY a").is_empty());
}

#[test]
fn order_by_in_line_comment_not_flagged() {
    assert!(check("SELECT a FROM t ORDER BY a -- ORDER BY 1").is_empty());
}

#[test]
fn correct_line_number() {
    let sql = "SELECT a\nFROM t\nORDER BY 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}

#[test]
fn correct_col_number() {
    // "SELECT a FROM t ORDER BY 1"
    //  col:                     ^26
    let diags = check("SELECT a FROM t ORDER BY 1");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 26);
}

#[test]
fn multi_digit_integer_flagged() {
    let diags = check("SELECT a FROM t ORDER BY 10");
    assert_eq!(diags.len(), 1);
}

#[test]
fn col1_identifier_not_flagged() {
    assert!(check("SELECT col1 FROM t ORDER BY col1").is_empty());
}
