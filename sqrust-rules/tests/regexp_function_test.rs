use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::regexp_function::RegexpFunction;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    RegexpFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(RegexpFunction.name(), "Ambiguous/RegexpFunction");
}

#[test]
fn regexp_like_violation() {
    let diags = check("SELECT * FROM t WHERE REGEXP_LIKE(col, 'pattern')");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/RegexpFunction");
}

#[test]
fn regexp_contains_violation() {
    let diags = check("SELECT * FROM t WHERE REGEXP_CONTAINS(col, r'pattern')");
    assert_eq!(diags.len(), 1);
}

#[test]
fn regexp_match_violation() {
    let diags = check("SELECT REGEXP_MATCH(col, 'pat') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn regexp_substr_violation() {
    let diags = check("SELECT REGEXP_SUBSTR(col, 'pat') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn rlike_violation() {
    let diags = check("SELECT * FROM t WHERE col RLIKE 'pattern'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn like_no_violation() {
    let diags = check("SELECT * FROM t WHERE col LIKE '%pattern%'");
    assert!(diags.is_empty());
}

#[test]
fn regexp_like_case_insensitive() {
    let diags = check("SELECT * FROM t WHERE regexp_like(col, 'p')");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_regexp_functions_multiple_violations() {
    let sql = "SELECT REGEXP_MATCH(a, 'p'), REGEXP_SUBSTR(b, 'q') FROM t WHERE REGEXP_LIKE(c, 'r')";
    let diags = check(sql);
    assert_eq!(diags.len(), 3);
}

#[test]
fn regexp_in_string_no_violation() {
    let diags = check("SELECT 'REGEXP_LIKE example' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn regexp_in_comment_no_violation() {
    let diags = check("-- REGEXP_LIKE\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn message_content() {
    let diags = check("SELECT * FROM t WHERE REGEXP_LIKE(col, 'p')");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("dialect"),
        "Expected message to mention 'dialect', got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM t WHERE REGEXP_LIKE(col, 'p')");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn regexp_count_violation() {
    let diags = check("SELECT REGEXP_COUNT(col, 'p') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn regexp_extract_violation() {
    let diags = check("SELECT REGEXP_EXTRACT(col, 'p') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn regexp_matches_violation() {
    let diags = check("SELECT REGEXP_MATCHES(col, 'p') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn regexp_instr_violation() {
    let diags = check("SELECT REGEXP_INSTR(col, 'p') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn regexp_split_to_array_violation() {
    let diags = check("SELECT REGEXP_SPLIT_TO_ARRAY(col, ',') FROM t");
    assert_eq!(diags.len(), 1);
}
