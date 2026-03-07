use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::duplicate_condition::DuplicateCondition;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DuplicateCondition.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DuplicateCondition.name(), "Lint/DuplicateCondition");
}

#[test]
fn no_duplicate_no_violation() {
    let sql = "SELECT * FROM t WHERE a = 1 AND b = 2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn duplicate_and_condition_one_violation() {
    let sql = "SELECT * FROM t WHERE a = 1 AND b = 2 AND a = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn duplicate_or_condition_one_violation() {
    let sql = "SELECT * FROM t WHERE a = 1 OR b = 2 OR a = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_insensitive_duplicate_flagged() {
    let sql = "SELECT * FROM t WHERE A = 1 AND a = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn whitespace_normalized_duplicate_flagged() {
    let sql = "SELECT * FROM t WHERE a  =  1 AND a = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_duplicate_different_values_no_violation() {
    let sql = "SELECT * FROM t WHERE a = 1 AND a = 2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn single_condition_no_violation() {
    let sql = "SELECT * FROM t WHERE a = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn no_where_clause_no_violation() {
    let sql = "SELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn duplicate_in_having_flagged() {
    let sql = "SELECT dept, COUNT(*) FROM emp GROUP BY dept HAVING count(*) > 0 AND count(*) > 0";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_mentions_duplicate() {
    let sql = "SELECT * FROM t WHERE a = 1 AND a = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_lowercase().contains("duplicate"),
        "message should mention 'duplicate': {}",
        diags[0].message
    );
}

#[test]
fn line_nonzero() {
    let sql = "SELECT * FROM t WHERE a = 1 AND a = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

#[test]
fn col_nonzero() {
    let sql = "SELECT * FROM t WHERE a = 1 AND a = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}
