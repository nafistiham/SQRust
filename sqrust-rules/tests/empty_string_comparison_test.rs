use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::empty_string_comparison::EmptyStringComparison;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    EmptyStringComparison.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(EmptyStringComparison.name(), "Lint/EmptyStringComparison");
}

#[test]
fn parse_error_returns_no_violations() {
    // Even with a parse error the text-based rule still scans; we just
    // ensure it doesn't panic and the diagnostic count is sane.
    // An empty/invalid query produces no matches.
    let sql = "";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn eq_empty_string_one_violation() {
    let sql = "SELECT * FROM t WHERE col = ''";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn neq_empty_string_one_violation() {
    let sql = "SELECT * FROM t WHERE col != ''";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn diamond_neq_empty_string_one_violation() {
    let sql = "SELECT * FROM t WHERE col <> ''";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn eq_non_empty_string_no_violation() {
    let sql = "SELECT * FROM t WHERE col = 'value'";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn eq_null_no_violation() {
    let sql = "SELECT * FROM t WHERE col = NULL";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn eq_empty_string_in_line_comment_no_violation() {
    let sql = "SELECT * FROM t -- WHERE col = ''\nWHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn eq_empty_string_in_block_comment_no_violation() {
    let sql = "SELECT * FROM t /* WHERE col = '' */ WHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiple_empty_string_comparisons_multiple_violations() {
    let sql = "SELECT * FROM t WHERE a = '' AND b != '' AND c <> ''";
    let diags = check(sql);
    assert_eq!(diags.len(), 3);
}

#[test]
fn escaped_quote_inside_string_no_violation() {
    // 'it''s' is an escaped single quote inside a non-empty string — not empty
    let sql = "SELECT * FROM t WHERE col = 'it''s'";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn line_col_points_to_operator() {
    // = is at col 27 on line 1 (1-indexed)
    let sql = "SELECT * FROM t WHERE col = ''";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col > 0);
    assert!(diags[0].line > 0);
    // The = operator is after "col "
    assert_eq!(diags[0].line, 1);
}

#[test]
fn message_format_is_correct() {
    let sql = "SELECT * FROM t WHERE col = ''";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "Comparison with empty string; consider checking for NULL as well"
    );
}

#[test]
fn eq_empty_string_lowercase_one_violation() {
    let sql = "select * from t where col = ''";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn eq_operator_col_position_is_correct() {
    // "WHERE col = ''" — col is at 1-indexed position 27
    // Verify the exact column reported matches the = sign
    let sql = "SELECT * FROM t WHERE col = ''";
    //                                     ^ col 27
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 27);
}
