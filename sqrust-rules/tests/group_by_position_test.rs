use sqrust_core::FileContext;
use sqrust_rules::ambiguous::group_by_position::GroupByPosition;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    GroupByPosition.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(GroupByPosition.name(), "Ambiguous/GroupByPosition");
}

#[test]
fn column_name_no_violation() {
    assert!(check("SELECT a, COUNT(*) FROM t GROUP BY a").is_empty());
}

#[test]
fn single_positional_ref_flagged() {
    let diags = check("SELECT 1, COUNT(*) FROM t GROUP BY 1");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/GroupByPosition");
}

#[test]
fn two_positional_refs_flagged() {
    let diags = check("SELECT a, b FROM t GROUP BY 1, 2");
    assert_eq!(diags.len(), 2);
}

#[test]
fn mixed_positional_and_column_only_integer_flagged() {
    let diags = check("SELECT a, b, c FROM t GROUP BY a, 2, c");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 35);
}

#[test]
fn group_by_in_string_not_flagged() {
    assert!(check("SELECT 'GROUP BY 1' FROM t GROUP BY a").is_empty());
}

#[test]
fn group_by_in_line_comment_not_flagged() {
    assert!(check("SELECT a FROM t GROUP BY a -- GROUP BY 1").is_empty());
}

#[test]
fn correct_line_number_when_group_by_on_line_2() {
    let sql = "SELECT a, COUNT(*)\nFROM t\nGROUP BY 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}

#[test]
fn correct_col_number_of_integer() {
    // "SELECT a FROM t GROUP BY 1"
    //  col:                     ^26
    let diags = check("SELECT a FROM t GROUP BY 1");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 26);
}

#[test]
fn multi_digit_integer_flagged() {
    let diags = check("SELECT a FROM t GROUP BY 10");
    assert_eq!(diags.len(), 1);
}

#[test]
fn alphanumeric_column_not_flagged() {
    assert!(check("SELECT a1 FROM t GROUP BY a1").is_empty());
}

#[test]
fn multiline_group_by_positional_ref_on_next_line() {
    let sql = "SELECT a, b\nFROM t\nGROUP BY\n  1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 4);
}

#[test]
fn correct_message_text() {
    let diags = check("SELECT a FROM t GROUP BY 1");
    assert_eq!(
        diags[0].message,
        "Avoid positional GROUP BY references; use column names"
    );
}
