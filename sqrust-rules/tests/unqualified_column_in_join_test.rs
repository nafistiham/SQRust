use sqrust_core::{FileContext, Rule};
use sqrust_rules::structure::unqualified_column_in_join::UnqualifiedColumnInJoin;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    UnqualifiedColumnInJoin.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnqualifiedColumnInJoin.name(), "Structure/UnqualifiedColumnInJoin");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn single_table_no_join_no_violation() {
    assert!(check("SELECT id, name FROM t WHERE id > 1").is_empty());
}

#[test]
fn all_qualified_with_join_no_violation() {
    assert!(check("SELECT t.id, u.name FROM t JOIN u ON t.id = u.t_id WHERE t.id > 1").is_empty());
}

#[test]
fn unqualified_select_col_with_join_flagged() {
    let d = check("SELECT id, name FROM t JOIN u ON t.id = u.t_id");
    assert!(!d.is_empty());
}

#[test]
fn unqualified_where_col_with_join_flagged() {
    let d = check("SELECT t.id FROM t JOIN u ON t.id = u.t_id WHERE id > 5");
    assert!(!d.is_empty());
}

#[test]
fn wildcard_select_no_violation() {
    // SELECT * is not a column ref — don't flag
    assert!(check("SELECT * FROM t JOIN u ON t.id = u.t_id").is_empty());
}

#[test]
fn count_star_no_violation() {
    // COUNT(*) — the * inside is not a column ref
    assert!(check("SELECT COUNT(*) FROM t JOIN u ON t.id = u.t_id").is_empty());
}

#[test]
fn message_mentions_qualify() {
    let d = check("SELECT id FROM t JOIN u ON t.id = u.t_id");
    assert!(!d.is_empty());
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("qualif") || msg.contains("table") || msg.contains("alias"),
        "expected message to mention qualifying columns, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM t JOIN u ON t.id = u.t_id");
    assert!(!d.is_empty());
    assert_eq!(d[0].rule, "Structure/UnqualifiedColumnInJoin");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM t JOIN u ON t.id = u.t_id");
    assert!(!d.is_empty());
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn function_arg_unqualified_flagged() {
    let d = check("SELECT UPPER(name) FROM t JOIN u ON t.id = u.t_id");
    assert!(!d.is_empty());
}

#[test]
fn on_clause_not_flagged() {
    // ON clause qualifications are expected and not flagged by this rule
    // (they must be qualified for the JOIN to make sense)
    // This test just ensures the rule doesn't double-count ON cols
    let d = check("SELECT t.id FROM t JOIN u ON t.id = u.t_id");
    assert!(d.is_empty());
}
