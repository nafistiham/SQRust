use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::keyword_identifier::KeywordIdentifier;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    KeywordIdentifier.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(KeywordIdentifier.name(), "Lint/KeywordIdentifier");
}

#[test]
fn name_column_one_violation() {
    let sql = "SELECT name FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("name"));
}

#[test]
fn safe_column_no_violation() {
    let sql = "SELECT col FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn compound_identifier_name_one_violation() {
    let sql = "SELECT t.name FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("name"));
}

#[test]
fn status_column_one_violation() {
    let sql = "SELECT status FROM users";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("status"));
}

#[test]
fn quoted_name_no_violation() {
    // Quoted identifier — intentional, not a violation
    let sql = r#"SELECT "name" FROM t"#;
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn type_column_one_violation() {
    let sql = "SELECT type FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("type"));
}

#[test]
fn user_as_alias_one_violation() {
    // sqlparser parses bare `user` as a zero-arg function call, not an Identifier,
    // so we detect the keyword via alias usage instead.
    let sql = "SELECT a AS user FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("user"));
}

#[test]
fn value_column_one_violation() {
    let sql = "SELECT value FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("value"));
}

#[test]
fn alias_is_keyword_one_violation() {
    let sql = "SELECT a AS name FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("name"));
}

#[test]
fn safe_alias_no_violation() {
    let sql = "SELECT a AS col_name FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn id_column_no_violation() {
    // 'id' is not a SQL keyword — should be fine
    let sql = "SELECT id FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn parse_error_returns_empty() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = KeywordIdentifier.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn multiple_keyword_columns_multiple_violations() {
    // name and value are both keywords
    let sql = "SELECT col1, col2, name, value FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_format_is_correct() {
    let sql = "SELECT status FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "'status' is a SQL keyword used as an identifier — consider renaming or quoting it"
    );
}
