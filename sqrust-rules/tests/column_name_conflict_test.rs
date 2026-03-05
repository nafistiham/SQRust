use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::column_name_conflict::ColumnNameConflict;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ColumnNameConflict.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ColumnNameConflict.name(), "Ambiguous/ColumnNameConflict");
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECT FROM FROM GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = ColumnNameConflict.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn distinct_columns_no_violation() {
    let diags = check("SELECT a, b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn duplicate_bare_column_one_violation() {
    // SELECT a, a FROM t — name 'a' appears twice
    let diags = check("SELECT a, a FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn duplicate_alias_one_violation() {
    // SELECT a AS x, b AS x FROM t — alias 'x' appears twice
    let diags = check("SELECT a AS x, b AS x FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn duplicate_alias_message_contains_name() {
    let diags = check("SELECT a AS x, b AS x FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('x'),
        "message should contain the duplicate name, got: {}",
        diags[0].message
    );
}

#[test]
fn compound_identifier_conflict_one_violation() {
    // SELECT a.name, b.name FROM t JOIN u — 'name' appears twice
    let diags = check("SELECT a.name, b.name FROM t JOIN u ON t.id = u.id");
    assert_eq!(diags.len(), 1);
}

#[test]
fn alias_and_compound_identifier_conflict_one_violation() {
    // SELECT a AS name, b.name FROM t JOIN u — 'name' appears twice
    let diags = check("SELECT a AS name, b.name FROM t JOIN u ON t.id = u.id");
    assert_eq!(diags.len(), 1);
}

#[test]
fn wildcard_no_violation() {
    // SELECT *, a FROM t — wildcard is skipped
    let diags = check("SELECT *, a FROM t");
    assert!(diags.is_empty());
}

#[test]
fn distinct_aliases_no_violation() {
    let diags = check("SELECT a AS x, b AS y FROM t");
    assert!(diags.is_empty());
}

#[test]
fn triple_duplicate_one_violation_reported_once() {
    // SELECT a, a, a FROM t — name 'a' duplicated but reported only once
    let diags = check("SELECT a, a, a FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_insensitive_conflict_one_violation() {
    // SELECT A, a FROM t — case-insensitive comparison
    let diags = check("SELECT A, a FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn unnamed_expressions_no_violation() {
    // SELECT 1, 2 — no predictable output name
    let diags = check("SELECT 1, 2");
    assert!(diags.is_empty());
}

#[test]
fn rule_assigned_to_diagnostic() {
    let diags = check("SELECT a, a FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/ColumnNameConflict");
}

#[test]
fn subquery_with_conflict_one_violation() {
    // Subquery has SELECT a, a — should be flagged
    let diags = check("SELECT x FROM (SELECT a, a FROM t) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn compound_identifier_alias_conflict_one_violation() {
    // SELECT t.a AS x, u.x FROM t JOIN u ON t.id = u.id — 'x' appears twice
    let diags = check("SELECT t.a AS x, u.x FROM t JOIN u ON t.id = u.id");
    assert_eq!(diags.len(), 1);
}
