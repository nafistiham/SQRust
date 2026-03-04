use sqrust_core::FileContext;
use sqrust_rules::capitalisation::functions::Functions;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    Functions.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(Functions.name(), "Capitalisation/Functions");
}

#[test]
fn uppercase_count_no_violation() {
    assert!(check("SELECT COUNT(*) FROM users").is_empty());
}

#[test]
fn lowercase_count_flagged() {
    let diags = check("SELECT count(*) FROM users");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Capitalisation/Functions");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 8);
}

#[test]
fn mixed_case_count_flagged() {
    let diags = check("SELECT Count(*) FROM users");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 8);
}

#[test]
fn count_without_paren_not_flagged() {
    // "count" as a bare word without "(" is not a function call
    assert!(check("SELECT count FROM users").is_empty());
}

#[test]
fn function_inside_single_quoted_string_skipped() {
    assert!(check("SELECT 'count(' FROM users").is_empty());
}

#[test]
fn function_inside_line_comment_skipped() {
    assert!(check("SELECT COUNT(*) FROM users -- count(*) again").is_empty());
}

#[test]
fn function_inside_block_comment_skipped() {
    assert!(check("SELECT COUNT(*) /* count(*) */ FROM users").is_empty());
}

#[test]
fn function_inside_double_quoted_identifier_skipped() {
    assert!(check(r#"SELECT "count(" FROM users"#).is_empty());
}

#[test]
fn function_inside_backtick_skipped() {
    assert!(check("SELECT `count(` FROM users").is_empty());
}

#[test]
fn lowercase_sum_flagged() {
    let diags = check("SELECT sum(amount) FROM orders");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 8);
}

#[test]
fn uppercase_coalesce_no_violation() {
    assert!(check("SELECT COALESCE(a, b) FROM t").is_empty());
}

#[test]
fn lowercase_coalesce_flagged() {
    let diags = check("SELECT coalesce(a, b) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_lowercase_functions_all_flagged() {
    let diags = check("SELECT count(*), sum(x) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn function_message_format() {
    let diags = check("SELECT count(*) FROM users");
    assert_eq!(
        diags[0].message,
        "Function 'count' should be UPPERCASE (use 'COUNT')"
    );
}

#[test]
fn function_on_second_line_correct_line_number() {
    let diags = check("SELECT\n  count(*)\nFROM users");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
    assert_eq!(diags[0].col, 3);
}

#[test]
fn uppercase_max_and_min_no_violation() {
    assert!(check("SELECT MAX(price), MIN(price) FROM products").is_empty());
}

#[test]
fn lowercase_max_flagged() {
    let diags = check("SELECT max(price) FROM products");
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_row_number_flagged() {
    let diags = check("SELECT row_number() OVER (PARTITION BY id)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn uppercase_row_number_no_violation() {
    assert!(check("SELECT ROW_NUMBER() OVER (PARTITION BY id)").is_empty());
}
