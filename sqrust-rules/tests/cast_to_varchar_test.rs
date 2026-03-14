use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::cast_to_varchar::CastToVarchar;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CastToVarchar.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CastToVarchar.name(), "Ambiguous/CastToVarchar");
}

#[test]
fn cast_to_varchar_no_length_violation() {
    let diags = check("SELECT CAST(x AS VARCHAR) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/CastToVarchar");
}

#[test]
fn cast_to_varchar_with_length_no_violation() {
    let diags = check("SELECT CAST(x AS VARCHAR(255)) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn cast_to_varchar_255_no_violation() {
    let diags = check("SELECT CAST(x AS VARCHAR(100)) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn cast_to_text_no_violation() {
    let diags = check("SELECT CAST(x AS TEXT) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn cast_to_int_no_violation() {
    let diags = check("SELECT CAST(x AS INTEGER) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn cast_to_nvarchar_no_length_violation() {
    let diags = check("SELECT CAST(x AS NVARCHAR) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cast_to_varchar_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE CAST(id AS VARCHAR) = '123'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cast_to_varchar_in_cte_violation() {
    let sql = "WITH cte AS (SELECT CAST(id AS VARCHAR) AS str_id FROM t) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_casts_multiple_violations() {
    let diags = check("SELECT CAST(a AS VARCHAR), CAST(b AS VARCHAR) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn parse_error_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = CastToVarchar.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn message_mentions_length() {
    let diags = check("SELECT CAST(x AS VARCHAR) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_lowercase().contains("length"),
        "Expected message to mention 'length', got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT CAST(x AS VARCHAR) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn cast_to_char_no_length_violation() {
    let diags = check("SELECT CAST(x AS CHAR) FROM t");
    assert_eq!(diags.len(), 1);
}
