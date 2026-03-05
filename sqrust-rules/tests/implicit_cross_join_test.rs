use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::implicit_cross_join::ImplicitCrossJoin;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ImplicitCrossJoin.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ImplicitCrossJoin.name(), "Ambiguous/ImplicitCrossJoin");
}

#[test]
fn two_comma_tables_one_violation() {
    let diags = check("SELECT * FROM a, b");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/ImplicitCrossJoin");
}

#[test]
fn three_comma_tables_one_violation_per_select() {
    // The plan says: report once per SELECT, not once per extra table.
    let diags = check("SELECT * FROM a, b, c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn inner_join_no_violation() {
    let diags = check("SELECT * FROM a INNER JOIN b ON a.id = b.id");
    assert!(diags.is_empty());
}

#[test]
fn single_table_no_violation() {
    let diags = check("SELECT * FROM a");
    assert!(diags.is_empty());
}

#[test]
fn left_join_no_violation() {
    let diags = check("SELECT * FROM a LEFT JOIN b ON a.id = b.id");
    assert!(diags.is_empty());
}

#[test]
fn explicit_cross_join_no_violation() {
    let diags = check("SELECT * FROM a CROSS JOIN b");
    assert!(diags.is_empty());
}

#[test]
fn subquery_with_implicit_cross_join_one_violation() {
    let diags = check("SELECT * FROM (SELECT * FROM a, b) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn comma_tables_with_where_one_violation() {
    let diags = check("SELECT * FROM a, b WHERE a.id = b.id");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cte_with_implicit_cross_join_detected() {
    let sql = "WITH cte AS (SELECT * FROM a, b) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_format_is_correct() {
    let diags = check("SELECT * FROM a, b");
    assert_eq!(
        diags[0].message,
        "Implicit cross join from comma-separated tables; use explicit JOIN syntax"
    );
}

#[test]
fn parse_error_returns_empty() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = ImplicitCrossJoin.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn union_first_select_with_comma_tables_one_violation() {
    let diags = check("SELECT * FROM a, b UNION SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn union_both_selects_with_comma_tables_two_violations() {
    let diags = check("SELECT * FROM a, b UNION SELECT * FROM c, d");
    assert_eq!(diags.len(), 2);
}
