use sqrust_core::FileContext;
use sqrust_rules::convention::colon_cast::ColonCast;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ColonCast.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ColonCast.name(), "Convention/ColonCast");
}

#[test]
fn colon_cast_one_violation() {
    let diags = check("SELECT id::TEXT FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cast_function_no_violation() {
    let diags = check("SELECT CAST(id AS TEXT) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn double_colon_in_string_no_violation() {
    let diags = check("SELECT '::' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn two_colon_casts_two_violations() {
    let diags = check("SELECT id::INT, name::TEXT FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn colon_cast_in_where_violation() {
    let diags = check("SELECT id FROM t WHERE age::INT > 18");
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_cast_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT id::TEXT FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    assert!(
        msg.contains("::") || msg.to_lowercase().contains("cast") || msg.to_lowercase().contains("portab"),
        "message should mention :: cast or portability, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT id::TEXT FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn col_points_to_double_colon() {
    // "SELECT id::TEXT FROM t"
    //  123456789
    // 'i' in id is col 8, '::' starts at col 10
    let diags = check("SELECT id::TEXT FROM t");
    assert!(!diags.is_empty());
    assert_eq!(diags[0].col, 10, "col should point at the first ':' of '::'");
}

#[test]
fn text_based_parse_error_still_scans() {
    // ColonCast is text-based; parse errors do not prevent scanning.
    let diags = check("SELECT id::TEXT FROM !!!invalid sql");
    // At minimum the ::TEXT cast must be detected.
    assert!(!diags.is_empty());
}

#[test]
fn colon_cast_date_type_violation() {
    let diags = check("SELECT ts::DATE FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn colon_cast_interval_violation() {
    let diags = check("SELECT '1 day'::INTERVAL FROM dual");
    assert_eq!(diags.len(), 1);
}
