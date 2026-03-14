use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::union_branch_limit::UnionBranchLimit;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    UnionBranchLimit.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnionBranchLimit.name(), "Structure/UnionBranchLimit");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = UnionBranchLimit.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn limit_in_first_branch_one_violation() {
    // Parenthesized branch with LIMIT — detectable by AST
    let diags = check("(SELECT a FROM t LIMIT 10) UNION ALL SELECT a FROM s");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/UnionBranchLimit");
}

#[test]
fn limit_on_outer_query_no_violation() {
    // LIMIT at end of full UNION — applies to outer query, not a branch
    let diags = check("SELECT a FROM t UNION ALL SELECT a FROM s LIMIT 10");
    assert!(diags.is_empty());
}

#[test]
fn no_limit_no_violation() {
    let diags = check("SELECT a FROM t UNION ALL SELECT a FROM s");
    assert!(diags.is_empty());
}

#[test]
fn limit_in_second_branch_one_violation() {
    // Right branch has LIMIT
    let diags = check("SELECT a FROM t UNION ALL (SELECT a FROM s LIMIT 5)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn limit_in_both_branches_two_violations() {
    // Both branches parenthesized with LIMIT
    let diags = check("(SELECT a FROM t LIMIT 10) UNION ALL (SELECT a FROM s LIMIT 5)");
    assert_eq!(diags.len(), 2);
}

#[test]
fn single_select_with_limit_no_violation() {
    let diags = check("SELECT a FROM t LIMIT 10");
    assert!(diags.is_empty());
}

#[test]
fn union_all_with_outer_limit_no_violation() {
    let diags = check("SELECT a FROM t UNION ALL SELECT b FROM s LIMIT 100");
    assert!(diags.is_empty());
}

#[test]
fn intersect_with_branch_limit_one_violation() {
    let sql = "(SELECT a FROM t LIMIT 3) INTERSECT SELECT a FROM s";
    let ctx = FileContext::from_source(sql, "test.sql");
    if ctx.parse_errors.is_empty() {
        let diags = UnionBranchLimit.check(&ctx);
        assert_eq!(diags.len(), 1, "INTERSECT with branch LIMIT should be flagged");
    }
}

#[test]
fn except_with_branch_limit_one_violation() {
    let sql = "(SELECT a FROM t LIMIT 5) EXCEPT SELECT a FROM s";
    let ctx = FileContext::from_source(sql, "test.sql");
    if ctx.parse_errors.is_empty() {
        let diags = UnionBranchLimit.check(&ctx);
        assert_eq!(diags.len(), 1, "EXCEPT with branch LIMIT should be flagged");
    }
}

#[test]
fn limit_in_branch_inside_cte_detected() {
    let sql = "
        WITH combined AS (
            (SELECT a FROM t LIMIT 10) UNION ALL SELECT a FROM s
        )
        SELECT a FROM combined
    ";
    let diags = check(sql);
    assert_eq!(diags.len(), 1, "LIMIT in branch inside CTE should be detected");
}

#[test]
fn limit_in_branch_inside_subquery_detected() {
    let sql = "
        SELECT a FROM (
            (SELECT a FROM t LIMIT 10) UNION ALL SELECT a FROM s
        ) sub
    ";
    let diags = check(sql);
    assert_eq!(diags.len(), 1, "LIMIT in branch inside subquery should be detected");
}

#[test]
fn message_mentions_limit_or_union() {
    let diags = check("(SELECT a FROM t LIMIT 10) UNION ALL SELECT a FROM s");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("limit") || msg.contains("union") || msg.contains("branch"),
        "expected message to mention 'limit', 'union', or 'branch', got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("(SELECT a FROM t LIMIT 10) UNION ALL SELECT a FROM s");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line must be >= 1");
    assert!(diags[0].col >= 1, "col must be >= 1");
}
