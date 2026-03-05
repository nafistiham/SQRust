use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::duplicate_alias::DuplicateAlias;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DuplicateAlias.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DuplicateAlias.name(), "Lint/DuplicateAlias");
}

#[test]
fn duplicate_alias_same_name_one_violation() {
    // SELECT col1 AS x, col2 AS x — both named 'x'
    let sql = "SELECT col1 AS x, col2 AS x FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("x"));
}

#[test]
fn no_duplicate_aliases_no_violation() {
    let sql = "SELECT col1 AS x, col2 AS y FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn duplicate_alias_case_insensitive_one_violation() {
    // 'x' and 'X' should be treated as the same alias
    let sql = "SELECT col1 AS x, col2 AS X FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.to_lowercase().contains("x"));
}

#[test]
fn no_aliases_no_violation() {
    let sql = "SELECT col1, col2 FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn three_cols_first_and_last_duplicate_one_violation() {
    // alias 'a' appears at positions 1 and 3
    let sql = "SELECT col1 AS a, col2 AS b, col3 AS a FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("a"));
}

#[test]
fn alias_appears_three_times_one_violation_reported() {
    // Report once even if alias appears 3 times
    let sql = "SELECT col1 AS a, col2 AS a, col3 AS a FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("a"));
}

#[test]
fn two_different_duplicate_aliases_two_violations() {
    // 'x' appears twice and 'y' appears twice — two separate violations
    let sql = "SELECT col1 AS x, col2 AS y, col3 AS x, col4 AS y FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn subquery_inner_duplicate_one_violation() {
    // Duplicate is inside the subquery's SELECT
    let sql = "SELECT * FROM (SELECT col1 AS x, col2 AS x FROM t) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("x"));
}

#[test]
fn outer_clean_inner_duplicate_only_inner_flagged() {
    // Outer SELECT has no duplicates; inner subquery does
    let sql = "SELECT a, b FROM (SELECT col1 AS p, col2 AS p FROM t) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("p"));
}

#[test]
fn outer_duplicate_and_inner_duplicate_both_flagged() {
    // Both outer SELECT and inner subquery have duplicate aliases
    let sql = "SELECT col1 AS z, col2 AS z FROM (SELECT col1 AS x, col2 AS x FROM t) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn parse_error_returns_empty() {
    // Intentionally broken SQL — parse_errors non-empty, rule must skip
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = DuplicateAlias.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn literal_aliases_no_duplicate_no_violation() {
    let sql = "SELECT 1 AS one, 2 AS two FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn cte_inner_select_duplicate_flagged() {
    // Duplicate inside the CTE body SELECT
    let sql = "WITH cte AS (SELECT col AS x, col2 AS x FROM t) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("x"));
}

#[test]
fn message_format_is_correct() {
    let sql = "SELECT col1 AS dup, col2 AS dup FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "Column alias 'dup' is used more than once in this SELECT"
    );
}
