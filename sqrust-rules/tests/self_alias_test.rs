use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::self_alias::SelfAlias;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SelfAlias.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelfAlias.name(), "Lint/SelfAlias");
}

#[test]
fn simple_self_alias_violation() {
    let sql = "SELECT col AS col FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("col"));
}

#[test]
fn table_qualified_self_alias_violation() {
    let sql = "SELECT t.col AS col FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_self_aliases_two_violations() {
    let sql = "SELECT a AS a, b AS b FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn different_alias_no_violation() {
    let sql = "SELECT col AS renamed FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn case_insensitive_self_alias_violation() {
    let sql = "SELECT COL AS col FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn non_alias_select_no_violation() {
    let sql = "SELECT col FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn expression_alias_no_violation() {
    let sql = "SELECT a + b AS total FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn parse_error_no_violation() {
    let sql = "SELECT FROM FROM broken !!!";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn empty_file_no_violation() {
    let sql = "";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn self_alias_one_col_among_many() {
    let sql = "SELECT a AS a, b AS renamed FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains('a'));
}

#[test]
fn star_no_violation() {
    let sql = "SELECT * FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn function_with_alias_no_violation() {
    let sql = "SELECT COUNT(*) AS count_all FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}
