use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::unaliased_expression::UnaliasedExpression;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    UnaliasedExpression.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnaliasedExpression.name(), "Ambiguous/UnaliasedExpression");
}

#[test]
fn arithmetic_without_alias_one_violation() {
    let diags = check("SELECT col1 + col2 FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/UnaliasedExpression");
}

#[test]
fn arithmetic_with_alias_no_violation() {
    let diags = check("SELECT col1 + col2 AS total FROM t");
    assert!(diags.is_empty());
}

#[test]
fn bare_column_no_violation() {
    let diags = check("SELECT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn qualified_column_no_violation() {
    let diags = check("SELECT t.col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn function_without_alias_one_violation() {
    let diags = check("SELECT UPPER(col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn function_with_alias_no_violation() {
    let diags = check("SELECT UPPER(col) AS upper_col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn case_without_alias_one_violation() {
    let diags = check("SELECT CASE WHEN x > 1 THEN 'yes' ELSE 'no' END FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cast_without_alias_one_violation() {
    let diags = check("SELECT CAST(col AS VARCHAR) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cast_with_alias_no_violation() {
    let diags = check("SELECT CAST(col AS VARCHAR) AS col_str FROM t");
    assert!(diags.is_empty());
}

#[test]
fn wildcard_no_violation() {
    let diags = check("SELECT * FROM t");
    assert!(diags.is_empty());
}

#[test]
fn literal_without_alias_one_violation() {
    let diags = check("SELECT 1 FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn literal_with_alias_no_violation() {
    let diags = check("SELECT 1 AS one FROM t");
    assert!(diags.is_empty());
}

#[test]
fn multiple_unaliased_expressions_multiple_violations() {
    let diags = check("SELECT col1 + col2, UPPER(col3) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_format_is_correct() {
    let diags = check("SELECT col1 + col2 FROM t");
    assert_eq!(
        diags[0].message,
        "Expression in SELECT should have an explicit alias"
    );
}

#[test]
fn parse_error_returns_empty() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = UnaliasedExpression.check(&ctx);
        assert!(diags.is_empty());
    }
}
