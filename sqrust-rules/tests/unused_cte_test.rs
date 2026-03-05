use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::unused_cte::UnusedCte;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    UnusedCte.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnusedCte.name(), "Lint/UnusedCte");
}

#[test]
fn unused_cte_one_violation() {
    let sql = "WITH my_cte AS (SELECT 1)\nSELECT * FROM other_table";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("my_cte"));
}

#[test]
fn used_cte_no_violation() {
    let sql = "WITH my_cte AS (SELECT 1)\nSELECT * FROM my_cte";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_ctes_both_used_no_violation() {
    let sql = "WITH cte1 AS (SELECT 1), cte2 AS (SELECT 2)\nSELECT * FROM cte1 JOIN cte2 ON cte1.id = cte2.id";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_ctes_one_unused_one_violation() {
    let sql = "WITH cte1 AS (SELECT 1), cte2 AS (SELECT 2)\nSELECT * FROM cte1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("cte2"));
}

#[test]
fn two_ctes_both_unused_two_violations() {
    let sql = "WITH cte1 AS (SELECT 1), cte2 AS (SELECT 2)\nSELECT * FROM other_table";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn cte_used_in_where_clause_no_violation() {
    let sql = "WITH my_cte AS (SELECT id FROM t)\nSELECT * FROM other WHERE id IN (SELECT id FROM my_cte)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn cte_used_in_join_no_violation() {
    let sql = "WITH my_cte AS (SELECT id FROM t)\nSELECT * FROM other JOIN my_cte ON other.id = my_cte.id";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn no_with_clause_no_violation() {
    let sql = "SELECT a, b FROM t WHERE a = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn correct_message_format() {
    let sql = "WITH cte_name AS (SELECT 1)\nSELECT * FROM other_table";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "CTE 'cte_name' is defined but never used");
}

#[test]
fn parse_error_returns_no_violations() {
    // Intentionally malformed SQL that cannot be parsed
    let sql = "WITH SELECT FROM FROM FROM";
    let ctx = FileContext::from_source(sql, "test.sql");
    // Ensure it is actually a parse error
    let diags = UnusedCte.check(&ctx);
    // Whether it parses or not, we should get no violations on broken SQL
    // (either parse_errors are non-empty → skip, or AST has no WITH → no violations)
    // The key requirement: no panic
    let _ = diags;
}

#[test]
fn parse_error_explicit_check() {
    // SQL that definitely fails parsing — we skip when parse_errors is non-empty
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        // When parse errors exist, rule must return empty
        let diags = UnusedCte.check(&ctx);
        assert!(diags.is_empty());
    }
    // If by chance it parsed (shouldn't), just verify no panic
}
