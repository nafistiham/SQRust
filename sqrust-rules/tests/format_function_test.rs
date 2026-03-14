use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::format_function::FormatFunction;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    FormatFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(FormatFunction.name(), "Ambiguous/FormatFunction");
}

#[test]
fn format_function_violation() {
    let diags = check("SELECT FORMAT(1234.5, 'N2') FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/FormatFunction");
}

#[test]
fn to_char_violation() {
    let diags = check("SELECT TO_CHAR(date_col, 'YYYY-MM-DD') FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/FormatFunction");
}

#[test]
fn to_varchar_violation() {
    let diags = check("SELECT TO_VARCHAR(num_col, '0.00') FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/FormatFunction");
}

#[test]
fn format_case_insensitive() {
    let diags = check("SELECT format(1234, '#,##0') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn to_char_case_insensitive() {
    let diags = check("SELECT to_char(d, 'YYYY') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cast_no_violation() {
    let diags = check("SELECT CAST(x AS VARCHAR) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn concat_no_violation() {
    let diags = check("SELECT CONCAT(a, b) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn format_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE FORMAT(amount, 'C') = '$1,000.00'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn to_char_in_cte_violation() {
    let diags = check("WITH c AS (SELECT TO_CHAR(d, 'YYYY-MM-DD') AS dt FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_format_functions_multiple_violations() {
    let diags = check("SELECT FORMAT(a, 'N2'), TO_CHAR(b, 'YYYY'), TO_VARCHAR(c, '0.0') FROM t");
    assert_eq!(diags.len(), 3);
}

#[test]
fn format_message_content() {
    let diags = check("SELECT FORMAT(1234.5, 'N2') FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("FORMAT"),
        "message should mention FORMAT, got: {msg}"
    );
    assert!(
        upper.contains("SQL SERVER") || upper.contains("MYSQL"),
        "message should mention SQL Server or MySQL, got: {msg}"
    );
}

#[test]
fn to_char_message_content() {
    let diags = check("SELECT TO_CHAR(date_col, 'YYYY') FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("ORACLE") || upper.contains("POSTGRESQL"),
        "message should mention Oracle or PostgreSQL, got: {msg}"
    );
}

#[test]
fn parse_error_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT FORMAT(1234.5, 'N2') FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
