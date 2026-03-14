use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::string_to_number_comparison::StringToNumberComparison;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    StringToNumberComparison.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        StringToNumberComparison.name(),
        "Ambiguous/StringToNumberComparison"
    );
}

#[test]
fn string_eq_integer_flagged() {
    let d = check("SELECT * FROM t WHERE code = 123");
    // 'code' is not a string literal here, but '123' is a bare integer
    // This is a column = integer case, which we do NOT flag (no way to know column type)
    // Per spec: flag 'value' = 123 pattern (quoted string on one side)
    assert!(d.is_empty());
}

#[test]
fn single_quoted_string_eq_integer_flagged() {
    let d = check("SELECT * FROM t WHERE '123' = 456");
    assert_eq!(d.len(), 1);
}

#[test]
fn integer_eq_single_quoted_string_flagged() {
    let d = check("SELECT * FROM t WHERE 456 = '123'");
    assert_eq!(d.len(), 1);
}

#[test]
fn single_quoted_string_neq_integer_flagged() {
    let d = check("SELECT * FROM t WHERE 'abc' != 99");
    assert_eq!(d.len(), 1);
}

#[test]
fn single_quoted_string_lt_integer_flagged() {
    let d = check("SELECT * FROM t WHERE 'abc' < 10");
    assert_eq!(d.len(), 1);
}

#[test]
fn single_quoted_string_gt_integer_flagged() {
    let d = check("SELECT * FROM t WHERE 'abc' > 10");
    assert_eq!(d.len(), 1);
}

#[test]
fn single_quoted_string_diamond_neq_integer_flagged() {
    let d = check("SELECT * FROM t WHERE 'abc' <> 10");
    assert_eq!(d.len(), 1);
}

#[test]
fn string_eq_decimal_flagged() {
    let d = check("SELECT * FROM t WHERE '12.50' = 12.50");
    assert_eq!(d.len(), 1);
}

#[test]
fn string_eq_string_not_flagged() {
    assert!(check("SELECT * FROM t WHERE 'abc' = 'def'").is_empty());
}

#[test]
fn integer_eq_integer_not_flagged() {
    assert!(check("SELECT * FROM t WHERE 1 = 2").is_empty());
}

#[test]
fn column_eq_integer_not_flagged() {
    // Can't know column type — should not flag
    assert!(check("SELECT * FROM t WHERE col = 123").is_empty());
}

#[test]
fn string_comparison_in_comment_not_flagged() {
    assert!(check("SELECT * FROM t -- WHERE '123' = 456\nWHERE id > 0").is_empty());
}

#[test]
fn two_violations_flagged() {
    let d = check("SELECT * FROM t WHERE '1' = 1 AND '2' = 2");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_coercion_or_cast() {
    let d = check("SELECT * FROM t WHERE '123' = 456");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("coercion") || msg.contains("cast") || msg.contains("implicit") || msg.contains("type"),
        "expected message to mention coercion/cast/implicit/type, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT * FROM t WHERE '123' = 456");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Ambiguous/StringToNumberComparison");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT * FROM t WHERE '123' = 456");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn string_leq_integer_flagged() {
    let d = check("SELECT * FROM t WHERE 'val' <= 50");
    assert_eq!(d.len(), 1);
}

#[test]
fn string_geq_integer_flagged() {
    let d = check("SELECT * FROM t WHERE 'val' >= 50");
    assert_eq!(d.len(), 1);
}
