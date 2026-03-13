use sqrust_core::FileContext;
use sqrust_rules::convention::explicit_join_type::ExplicitJoinType;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ExplicitJoinType.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ExplicitJoinType.name(), "Convention/ExplicitJoinType");
}

#[test]
fn bare_join_is_flagged() {
    let diags = check("SELECT a FROM t JOIN s ON t.id = s.id");
    assert_eq!(diags.len(), 1);
}

#[test]
fn inner_join_is_not_flagged() {
    let diags = check("SELECT a FROM t INNER JOIN s ON t.id = s.id");
    assert!(diags.is_empty());
}

#[test]
fn left_join_is_not_flagged() {
    let diags = check("SELECT a FROM t LEFT JOIN s ON t.id = s.id");
    assert!(diags.is_empty());
}

#[test]
fn left_outer_join_is_not_flagged() {
    let diags = check("SELECT a FROM t LEFT OUTER JOIN s ON t.id = s.id");
    assert!(diags.is_empty());
}

#[test]
fn right_join_is_not_flagged() {
    let diags = check("SELECT a FROM t RIGHT JOIN s ON t.id = s.id");
    assert!(diags.is_empty());
}

#[test]
fn full_outer_join_is_not_flagged() {
    let diags = check("SELECT a FROM t FULL OUTER JOIN s ON t.id = s.id");
    assert!(diags.is_empty());
}

#[test]
fn cross_join_is_not_flagged() {
    let diags = check("SELECT a FROM t CROSS JOIN s");
    assert!(diags.is_empty());
}

#[test]
fn natural_join_is_not_flagged() {
    let diags = check("SELECT a FROM t NATURAL JOIN s");
    assert!(diags.is_empty());
}

#[test]
fn multiple_bare_joins_flagged() {
    let sql = "SELECT a FROM t JOIN s ON t.id = s.id JOIN u ON t.id = u.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn mix_bare_and_inner_join_flags_one() {
    let sql = "SELECT a FROM t JOIN s ON t.id = s.id INNER JOIN u ON t.id = u.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn join_inside_string_not_flagged() {
    let sql = "SELECT 'JOIN' FROM t JOIN s ON t.id = s.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_source_still_runs() {
    // Syntactically broken SQL — source-level rule still runs
    let diags = check("SELECT FROM JOIN !!!");
    // Just check it doesn't panic; exact count varies with heuristics
    let _ = diags;
}

#[test]
fn lowercase_bare_join_is_flagged() {
    let diags = check("SELECT a FROM t join s ON t.id = s.id");
    assert_eq!(diags.len(), 1);
}

#[test]
fn bare_join_violation_message() {
    let diags = check("SELECT a FROM t JOIN s ON t.id = s.id");
    assert_eq!(
        diags[0].message,
        "Bare JOIN defaults to INNER JOIN — use INNER JOIN explicitly for clarity"
    );
}
