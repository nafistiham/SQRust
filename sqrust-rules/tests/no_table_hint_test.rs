use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::no_table_hint::NoTableHint;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoTableHint.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoTableHint.name(), "Convention/NoTableHint");
}

#[test]
fn nolock_violation() {
    let diags = check("SELECT * FROM t WITH (NOLOCK)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("NOLOCK"));
}

#[test]
fn readpast_violation() {
    let diags = check("SELECT * FROM t WITH (READPAST)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("READPAST"));
}

#[test]
fn updlock_violation() {
    let diags = check("SELECT * FROM t WITH (UPDLOCK)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("UPDLOCK"));
}

#[test]
fn holdlock_violation() {
    let diags = check("SELECT * FROM t WITH (HOLDLOCK)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("HOLDLOCK"));
}

#[test]
fn tablock_violation() {
    let diags = check("SELECT * FROM t WITH (TABLOCK)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("TABLOCK"));
}

#[test]
fn rowlock_violation() {
    let diags = check("SELECT * FROM t WITH (ROWLOCK)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("ROWLOCK"));
}

#[test]
fn nolock_lowercase_violation() {
    let diags = check("SELECT * FROM t WITH (nolock)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("nolock"));
}

#[test]
fn nolock_in_string_no_violation() {
    let diags = check("SELECT 'WITH (NOLOCK)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn nolock_in_comment_no_violation() {
    let diags = check("-- SELECT * FROM t WITH (NOLOCK)\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn plain_with_clause_no_violation() {
    let diags = check("WITH cte AS (SELECT 1) SELECT * FROM cte");
    assert!(diags.is_empty());
}

#[test]
fn no_hint_no_violation() {
    let diags = check("SELECT * FROM t WHERE a = 1");
    assert!(diags.is_empty());
}

#[test]
fn multiple_hints_two_violations() {
    let sql = "SELECT * FROM t WITH (NOLOCK) JOIN u WITH (NOLOCK) ON t.id = u.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_contains_isolation_level_hint() {
    let diags = check("SELECT * FROM t WITH (NOLOCK)");
    assert!(diags[0].message.contains("SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED"));
}
