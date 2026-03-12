use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::duplicate_select_column::DuplicateSelectColumn;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DuplicateSelectColumn.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DuplicateSelectColumn.name(), "Lint/DuplicateSelectColumn");
}

#[test]
fn duplicate_unnamed_column_one_violation() {
    // 'a' appears twice
    let sql = "SELECT a, b, a FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("a"));
}

#[test]
fn no_duplicates_no_violation() {
    let sql = "SELECT a, b, c FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn distinct_aliases_no_violation() {
    // Aliases are distinct — no violation
    let sql = "SELECT a AS p, b AS q FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn column_appears_three_times_one_violation() {
    // 'a' three times — report once (when count first hits 2)
    let sql = "SELECT a, a, a FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("a"));
}

#[test]
fn compound_identifier_last_part_checked() {
    // t.a and t.a — last part is 'a' in both
    let sql = "SELECT t.a, t.a FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("a"));
}

#[test]
fn duplicate_alias_one_violation() {
    // Both items aliased 'foo' — duplicate
    let sql = "SELECT a AS foo, b AS foo FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("foo"));
}

#[test]
fn wildcard_ignored() {
    // Wildcard should not be flagged
    let sql = "SELECT * FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn subquery_inner_duplicate_flagged() {
    // Duplicate column in the inner SELECT
    let sql = "SELECT x FROM (SELECT a, a FROM t) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("a"));
}

#[test]
fn cte_inner_duplicate_flagged() {
    // Duplicate inside CTE body
    let sql = "WITH c AS (SELECT a, a FROM t) SELECT * FROM c";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("a"));
}

#[test]
fn case_insensitive_duplicate() {
    // 'A' and 'a' are the same column name
    let sql = "SELECT A, a FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn distinct_aliases_are_clean() {
    let sql = "SELECT a AS x, b AS y FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn parse_error_returns_empty() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = DuplicateSelectColumn.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn multiple_statements_correct_count() {
    // First statement is clean, second has a duplicate
    let sql = "SELECT a, b FROM t; SELECT a, a FROM t;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_format_is_correct() {
    let sql = "SELECT dup, dup FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "Column 'dup' is selected more than once in this SELECT"
    );
}
