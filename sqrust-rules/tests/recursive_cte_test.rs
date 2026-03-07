use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::recursive_cte::RecursiveCte;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    RecursiveCte.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(RecursiveCte.name(), "Lint/RecursiveCte");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "WITH RECURSIVE @@##GARBAGE";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = RecursiveCte.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn with_recursive_one_violation() {
    let sql = "WITH RECURSIVE cte AS (SELECT 1) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn with_non_recursive_no_violation() {
    let sql = "WITH cte AS (SELECT 1) SELECT * FROM cte";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_recursive_ctes_one_statement_one_violation() {
    // One WITH RECURSIVE with multiple CTEs — still one statement, one violation.
    let sql = "WITH RECURSIVE \
                cte1 AS (SELECT 1 UNION ALL SELECT n+1 FROM cte1 WHERE n < 10), \
                cte2 AS (SELECT 2 UNION ALL SELECT n+1 FROM cte2 WHERE n < 5) \
               SELECT * FROM cte1, cte2";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_no_cte_no_violation() {
    let sql = "SELECT * FROM t WHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_recursive_or_loop() {
    let sql = "WITH RECURSIVE cte AS (SELECT 1) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message.to_lowercase();
    assert!(
        msg.contains("recursive") || msg.contains("loop"),
        "message '{}' should mention 'recursive' or 'loop'",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "WITH RECURSIVE cte AS (SELECT 1) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn lowercase_with_recursive_violation() {
    let sql = "with recursive cte as (select 1 union all select n+1 from cte where n < 10) select * from cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn recursive_cte_in_subquery_violation() {
    let sql = "SELECT * FROM (WITH RECURSIVE inner_cte AS (SELECT 1) SELECT * FROM inner_cte) sub";
    // Parser may or may not support CTE in subquery; if it does, flag it.
    // If parse_errors is non-empty we skip — same as other tests.
    let ctx = FileContext::from_source(sql, "test.sql");
    if ctx.parse_errors.is_empty() {
        let diags = RecursiveCte.check(&ctx);
        assert_eq!(diags.len(), 1);
    }
}

#[test]
fn correct_line_for_with_keyword() {
    let sql = "\nWITH RECURSIVE cte AS (SELECT 1) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn non_recursive_cte_with_multiple_ctes_no_violation() {
    let sql = "WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a, b";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn col_nonzero() {
    let sql = "  WITH RECURSIVE cte AS (SELECT 1) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn two_separate_recursive_statements_two_violations() {
    let sql = "WITH RECURSIVE cte AS (SELECT 1) SELECT * FROM cte;\nWITH RECURSIVE cte2 AS (SELECT 2) SELECT * FROM cte2";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}
