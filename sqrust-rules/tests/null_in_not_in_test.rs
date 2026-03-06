use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::null_in_not_in::NullInNotIn;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NullInNotIn.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NullInNotIn.name(), "Lint/NullInNotIn");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "NOT VALID SQL ###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = NullInNotIn.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn not_in_with_null_one_violation() {
    let sql = "SELECT * FROM t WHERE id NOT IN (1, NULL, 3)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_in_without_null_no_violation() {
    let sql = "SELECT * FROM t WHERE id NOT IN (1, 2, 3)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn in_with_null_no_violation() {
    // Positive IN with NULL — different semantics, not flagged
    let sql = "SELECT * FROM t WHERE id IN (1, NULL, 3)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn not_in_only_null_one_violation() {
    let sql = "SELECT * FROM t WHERE id NOT IN (NULL)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_in_null_in_where_violation() {
    let sql = "SELECT * FROM orders WHERE status NOT IN ('active', NULL)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_in_null_in_having_violation() {
    let sql = "SELECT dept, COUNT(*) FROM emp GROUP BY dept HAVING dept NOT IN ('hr', NULL)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_in_expression_no_violation() {
    let sql = "SELECT * FROM t WHERE id = 1 AND name = 'foo'";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_contains_useful_text() {
    let sql = "SELECT * FROM t WHERE id NOT IN (1, NULL)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("NULL"),
        "message should mention NULL: {}",
        diags[0].message
    );
    assert!(
        diags[0].message.to_lowercase().contains("not in")
            || diags[0].message.to_lowercase().contains("empty"),
        "message should be informative: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "SELECT * FROM t WHERE id NOT IN (1, NULL)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn two_not_in_null_two_violations() {
    let sql = "SELECT * FROM t WHERE id NOT IN (1, NULL) AND cat NOT IN ('a', NULL, 'b')";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn not_in_null_in_subquery_violation() {
    let sql = "SELECT * FROM t WHERE id IN (SELECT x FROM s WHERE x NOT IN (10, NULL, 20))";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_in_multiple_nulls_one_violation_per_expression() {
    // One NOT IN expression containing two NULLs — still one violation (one expression)
    let sql = "SELECT * FROM t WHERE id NOT IN (NULL, NULL)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
