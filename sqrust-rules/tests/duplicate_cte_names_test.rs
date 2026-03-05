use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::duplicate_cte_names::DuplicateCteNames;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DuplicateCteNames.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DuplicateCteNames.name(), "Lint/DuplicateCteNames");
}

#[test]
fn duplicate_cte_name_one_violation() {
    let sql = "WITH a AS (SELECT 1), a AS (SELECT 2) SELECT * FROM a";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn distinct_cte_names_no_violation() {
    let sql = "WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn duplicate_cte_name_case_insensitive_one_violation() {
    let sql = "WITH a AS (SELECT 1), A AS (SELECT 2) SELECT * FROM a";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_ctes_no_violation() {
    let sql = "SELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn single_cte_no_violation() {
    let sql = "WITH a AS (SELECT 1) SELECT * FROM a";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn three_ctes_first_and_third_same_name_one_violation() {
    let sql = "WITH a AS (SELECT 1), b AS (SELECT 2), a AS (SELECT 3) SELECT * FROM b";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_ctes_all_same_name_one_violation() {
    // Same name appears 3 times — still 1 diagnostic (one per duplicate name)
    let sql = "WITH a AS (SELECT 1), a AS (SELECT 2), a AS (SELECT 3) SELECT * FROM a";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_different_duplicate_names_two_violations() {
    let sql = "WITH a AS (SELECT 1), b AS (SELECT 2), a AS (SELECT 3), b AS (SELECT 4) SELECT * FROM a";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn nested_query_with_own_duplicate_ctes_detected() {
    // Outer has unique CTEs; inner subquery (in a CTE body) has its own WITH with a duplicate
    let sql = concat!(
        "WITH outer_cte AS (",
        "  SELECT * FROM (",
        "    WITH inner_a AS (SELECT 1), inner_a AS (SELECT 2) SELECT * FROM inner_a",
        "  ) sub",
        ") SELECT * FROM outer_cte"
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_empty() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = DuplicateCteNames.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn message_includes_duplicate_cte_name() {
    let sql = "WITH cte AS (SELECT 1), cte AS (SELECT 2) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("cte"));
}

#[test]
fn message_format_is_correct() {
    let sql = "WITH cte AS (SELECT 1), cte AS (SELECT 2) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "CTE name 'cte' is used more than once in this WITH clause"
    );
}
