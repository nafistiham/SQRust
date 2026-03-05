use sqrust_core::FileContext;
use sqrust_rules::layout::tab_indentation::TabIndentation;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    TabIndentation.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(TabIndentation.name(), "Layout/TabIndentation");
}

#[test]
fn empty_file_has_no_violations() {
    assert!(check("").is_empty());
}

#[test]
fn file_with_no_tabs_has_no_violations() {
    let diags = check("SELECT id\nFROM users\nWHERE id = 1\n");
    assert!(diags.is_empty());
}

#[test]
fn single_line_starting_with_tab_produces_one_violation() {
    let diags = check("\tSELECT id\n");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn multiple_leading_tabs_produce_one_violation_not_many() {
    let diags = check("\t\t\tSELECT id\n");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn mid_line_tab_does_not_produce_violation() {
    // Tab in the middle of a line (e.g., between columns) should not be flagged
    let diags = check("SELECT\ta\tFROM t\n");
    assert!(diags.is_empty());
}

#[test]
fn multiple_lines_with_leading_tabs_each_produce_one_violation() {
    let diags = check("\tSELECT id\n\tFROM users\n\tWHERE id = 1\n");
    assert_eq!(diags.len(), 3);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[1].line, 2);
    assert_eq!(diags[2].line, 3);
}

#[test]
fn mix_of_tabbed_and_non_tabbed_lines_only_flags_tabbed() {
    let diags = check("SELECT id\n\tFROM users\nWHERE id = 1\n");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn correct_message_text() {
    let diags = check("\tSELECT 1\n");
    assert_eq!(
        diags[0].message,
        "Avoid tab characters for indentation; use spaces"
    );
}

#[test]
fn correct_rule_name_in_diagnostic() {
    let diags = check("\tSELECT 1\n");
    assert_eq!(diags[0].rule, "Layout/TabIndentation");
}

#[test]
fn tab_only_line_produces_violation() {
    // A line that is only a tab character (empty indented line)
    let diags = check("\t\n");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 1);
}
