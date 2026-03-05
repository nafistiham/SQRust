use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::where_tautology::WhereTautology;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    WhereTautology.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(WhereTautology.name(), "Lint/WhereTautology");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = WhereTautology.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn where_one_equals_one_no_spaces_one_violation() {
    let sql = "SELECT * FROM t WHERE 1=1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn where_one_equals_one_with_spaces_one_violation() {
    let sql = "SELECT * FROM t WHERE 1 = 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn where_true_one_violation() {
    let sql = "SELECT * FROM t WHERE TRUE";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn where_true_lowercase_one_violation() {
    let sql = "SELECT * FROM t WHERE true";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn where_col_equals_one_no_violation() {
    let sql = "SELECT * FROM t WHERE col = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn where_one_equals_one_in_line_comment_no_violation() {
    let sql = "SELECT * FROM t -- WHERE 1=1\nWHERE col = 5";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn where_one_equals_one_in_block_comment_no_violation() {
    let sql = "SELECT * FROM t /* WHERE 1=1 */ WHERE col = 5";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn where_one_equals_one_in_string_literal_no_violation() {
    let sql = "SELECT 'WHERE 1=1' FROM t WHERE col = 5";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn and_one_equals_one_one_violation() {
    let sql = "SELECT * FROM t WHERE col = 5 AND 1=1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn and_true_one_violation() {
    let sql = "SELECT * FROM t WHERE col = 5 AND TRUE";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_tautologies_multiple_violations() {
    let sql = "SELECT * FROM t WHERE 1=1;\nSELECT * FROM u WHERE TRUE";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn where_one_equals_two_no_violation() {
    let sql = "SELECT * FROM t WHERE 1 = 2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_format_is_correct() {
    let sql = "SELECT * FROM t WHERE 1=1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "Tautological WHERE condition always evaluates to true"
    );
}

#[test]
fn line_and_col_are_nonzero() {
    let sql = "SELECT * FROM t WHERE 1=1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn where_one_equals_one_uppercase_keyword_one_violation() {
    let sql = "SELECT * FROM t WHERE 1=1 AND col = 5";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_where_clause_no_violation() {
    let sql = "SELECT * FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}
