use sqrust_core::FileContext;
use sqrust_rules::convention::comma_style::CommaStyle;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CommaStyle.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CommaStyle.name(), "Convention/CommaStyle");
}

#[test]
fn all_trailing_commas_no_violation() {
    let sql = "SELECT\n    id,\n    name,\n    email\nFROM users";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn all_leading_commas_no_violation() {
    let sql = "SELECT\n    id\n  , name\n  , email\nFROM users";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn mixed_trailing_and_leading_commas_flagged() {
    // Line 2 trailing, line 3 leading — mixed
    let sql = "SELECT\n    id,\n  , name\n    email\nFROM users";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_violation_points_to_first_inconsistent_line() {
    // trailing on line 2, leading on line 3
    let sql = "SELECT\n    id,\n  , name\n    email\nFROM users";
    let diags = check(sql);
    // First leading comma line is line 3
    assert_eq!(diags[0].line, 3);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn mixed_violation_has_correct_message() {
    let sql = "SELECT\n    id,\n  , name\n    email\nFROM users";
    let diags = check(sql);
    assert_eq!(
        diags[0].message,
        "Inconsistent comma style: mix of leading and trailing commas"
    );
}

#[test]
fn single_line_no_commas_no_violation() {
    let diags = check("SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn single_trailing_comma_only_no_violation() {
    let sql = "SELECT\n    id,\nFROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn single_leading_comma_only_no_violation() {
    let sql = "SELECT\n    id\n  , name\nFROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiple_mixed_only_one_diagnostic_produced() {
    // trailing on lines 2+4, leading on lines 3+5
    let sql = "SELECT\n    id,\n  , name,\n    email,\n  , phone\nFROM t";
    let diags = check(sql);
    // A line can't be both — "name," has trailing comma AND leading comma on same line.
    // This is a tricky case: leading AND trailing on same line.
    // Per spec, count "trailing" and "leading" across the file.
    // Line "  , name," — first non-ws is ',' (leading) AND last non-ws is ',' (trailing)
    // Spec: leading comma line = first non-ws is ','; trailing comma line = last non-ws is ','
    // If same line qualifies as both, it contributes to both counts.
    // The point: as long as there's mixing, exactly 1 diagnostic.
    assert_eq!(diags.len(), 1);
}
