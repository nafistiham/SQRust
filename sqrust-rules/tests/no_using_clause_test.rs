use sqrust_core::FileContext;
use sqrust_rules::convention::no_using_clause::NoUsingClause;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoUsingClause.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoUsingClause.name(), "Convention/NoUsingClause");
}

#[test]
fn parse_error_produces_no_violations() {
    let diags = check("SELECT FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn join_on_not_flagged() {
    let diags = check("SELECT * FROM t JOIN u ON t.id = u.id");
    assert!(diags.is_empty());
}

#[test]
fn inner_join_using_flagged() {
    let diags = check("SELECT * FROM t JOIN u USING (id)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn left_join_using_flagged() {
    let diags = check("SELECT * FROM t LEFT JOIN u USING (id)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn right_join_using_flagged() {
    let diags = check("SELECT * FROM t RIGHT JOIN u USING (id)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn full_join_using_flagged() {
    let diags = check("SELECT * FROM t FULL JOIN u USING (id)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cross_join_not_flagged() {
    // CROSS JOIN has no constraint at all
    let diags = check("SELECT * FROM t CROSS JOIN u");
    assert!(diags.is_empty());
}

#[test]
fn mixed_on_and_using_one_violation() {
    // Only the USING join should be flagged
    let diags = check("SELECT * FROM t JOIN u ON t.id = u.id JOIN v USING (vid)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_using_joins_multiple_violations() {
    let sql = "SELECT * FROM t JOIN u USING (id) JOIN v USING (vid)";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn natural_join_not_flagged() {
    // NATURAL JOIN uses JoinConstraint::Natural, not Using
    let diags = check("SELECT * FROM t NATURAL JOIN u");
    assert!(diags.is_empty());
}

#[test]
fn line_col_points_to_using_keyword() {
    // "SELECT * FROM t JOIN u USING (id)"
    //  1234567890123456789012345
    // 'U' of USING is at col 24
    let diags = check("SELECT * FROM t JOIN u USING (id)");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 24);
}

#[test]
fn message_format_correct() {
    let diags = check("SELECT * FROM t JOIN u USING (id)");
    assert_eq!(
        diags[0].message,
        "JOIN USING clause found; prefer explicit ON conditions for clarity"
    );
}

#[test]
fn using_in_subquery_flagged() {
    let sql = "SELECT * FROM (SELECT * FROM t JOIN u USING (id)) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
