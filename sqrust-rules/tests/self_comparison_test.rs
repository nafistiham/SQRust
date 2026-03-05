use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::self_comparison::SelfComparison;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SelfComparison.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelfComparison.name(), "Ambiguous/SelfComparison");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = SelfComparison.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn col_equals_col_one_violation() {
    let diags = check("SELECT * FROM t WHERE col = col");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/SelfComparison");
}

#[test]
fn different_columns_no_violation() {
    let diags = check("SELECT * FROM t WHERE a = b");
    assert!(diags.is_empty());
}

#[test]
fn qualified_same_table_same_col_one_violation() {
    let diags = check("SELECT * FROM t WHERE t.col = t.col");
    assert_eq!(diags.len(), 1);
}

#[test]
fn qualified_different_tables_no_violation() {
    let diags = check("SELECT * FROM t1, t2 WHERE t1.col = t2.col");
    assert!(diags.is_empty());
}

#[test]
fn case_insensitive_same_column_one_violation() {
    let diags = check("SELECT * FROM t WHERE Col = col");
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_equal_self_comparison_one_violation() {
    let diags = check("SELECT * FROM t WHERE col != col");
    assert_eq!(diags.len(), 1);
}

#[test]
fn less_than_self_comparison_one_violation() {
    let diags = check("SELECT * FROM t WHERE col < col");
    assert_eq!(diags.len(), 1);
}

#[test]
fn and_one_self_comparison_one_violation() {
    let diags = check("SELECT * FROM t WHERE a = a AND b = c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_self_comparisons_two_violations() {
    let diags = check("SELECT * FROM t WHERE a = a AND b = b");
    assert_eq!(diags.len(), 2);
}

#[test]
fn select_without_where_no_violation() {
    let diags = check("SELECT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn message_contains_column_name() {
    let diags = check("SELECT * FROM t WHERE mycolumn = mycolumn");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("mycolumn"),
        "expected message to contain 'mycolumn', got: {}",
        diags[0].message
    );
}

#[test]
fn nested_self_comparison_one_violation() {
    // (col) = (col) via Nested
    let diags = check("SELECT * FROM t WHERE (col) = (col)");
    assert_eq!(diags.len(), 1);
}
