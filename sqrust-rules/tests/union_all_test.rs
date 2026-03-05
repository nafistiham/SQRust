use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::union_all::UnionAll;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    UnionAll.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnionAll.name(), "Structure/UnionAll");
}

#[test]
fn bare_union_one_violation() {
    let diags = check("SELECT a FROM t UNION SELECT b FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("UNION ALL") || diags[0].message.contains("UNION DISTINCT"));
}

#[test]
fn union_all_no_violation() {
    let diags = check("SELECT a FROM t UNION ALL SELECT b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn union_distinct_no_violation() {
    let diags = check("SELECT a FROM t UNION DISTINCT SELECT b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn bare_union_lowercase_one_violation() {
    let diags = check("select a from t union select b from t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn union_all_lowercase_no_violation() {
    let diags = check("select a from t union all select b from t");
    assert!(diags.is_empty());
}

#[test]
fn multiple_bare_unions_two_violations() {
    let diags = check("SELECT a FROM t UNION SELECT b FROM t UNION SELECT c FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn mix_union_all_and_bare_union_one_violation() {
    let diags = check("SELECT a FROM t UNION ALL SELECT b FROM t UNION SELECT c FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn union_in_line_comment_no_violation() {
    let diags = check("SELECT a FROM t -- UNION SELECT b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn union_in_string_no_violation() {
    let diags = check("SELECT 'UNION' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn union_all_in_string_no_violation() {
    let diags = check("SELECT 'union all' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn union_newline_all_no_violation() {
    let diags = check("SELECT a FROM t UNION\nALL SELECT b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn union_newline_distinct_no_violation() {
    let diags = check("SELECT a FROM t UNION\nDISTINCT SELECT b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn violation_line_col_is_at_union_keyword() {
    let diags = check("SELECT a FROM t UNION SELECT b FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    // "SELECT a FROM t " is 16 chars, so UNION starts at col 17
    assert_eq!(diags[0].col, 17);
}
