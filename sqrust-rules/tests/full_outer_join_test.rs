use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::full_outer_join::FullOuterJoin;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    FullOuterJoin.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(FullOuterJoin.name(), "Ambiguous/FullOuterJoin");
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = FullOuterJoin.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn full_outer_join_one_violation() {
    let diags = check("SELECT * FROM a FULL OUTER JOIN b ON a.id = b.id");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/FullOuterJoin");
}

#[test]
fn full_join_one_violation() {
    // FULL JOIN is syntactic sugar for FULL OUTER JOIN — must also flag.
    let diags = check("SELECT * FROM a FULL JOIN b ON a.id = b.id");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/FullOuterJoin");
}

#[test]
fn inner_join_no_violation() {
    let diags = check("SELECT * FROM a INNER JOIN b ON a.id = b.id");
    assert!(diags.is_empty());
}

#[test]
fn left_join_no_violation() {
    let diags = check("SELECT * FROM a LEFT JOIN b ON a.id = b.id");
    assert!(diags.is_empty());
}

#[test]
fn right_join_no_violation() {
    let diags = check("SELECT * FROM a RIGHT JOIN b ON a.id = b.id");
    assert!(diags.is_empty());
}

#[test]
fn cross_join_no_violation() {
    let diags = check("SELECT * FROM a CROSS JOIN b");
    assert!(diags.is_empty());
}

#[test]
fn no_join_no_violation() {
    let diags = check("SELECT * FROM a");
    assert!(diags.is_empty());
}

#[test]
fn two_full_outer_joins_two_violations() {
    let diags = check(
        "SELECT * FROM a FULL OUTER JOIN b ON a.id = b.id FULL OUTER JOIN c ON a.id = c.id",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn full_outer_join_in_subquery_violation() {
    let diags = check(
        "SELECT * FROM (SELECT * FROM a FULL OUTER JOIN b ON a.id = b.id) sub",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT * FROM a FULL OUTER JOIN b ON a.id = b.id");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("FULL") || diags[0].message.contains("full"),
        "message was: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM a FULL OUTER JOIN b ON a.id = b.id");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn full_outer_join_in_cte_violation() {
    let sql =
        "WITH cte AS (SELECT * FROM a FULL OUTER JOIN b ON a.id = b.id) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
