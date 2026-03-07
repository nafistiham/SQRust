use sqrust_core::FileContext;
use sqrust_rules::convention::like_tautology::LikeTautology;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    LikeTautology.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(LikeTautology.name(), "Convention/LikeTautology");
}

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn like_percent_only_one_violation() {
    let diags = check("SELECT * FROM t WHERE name LIKE '%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn like_double_percent_one_violation() {
    let diags = check("SELECT * FROM t WHERE name LIKE '%%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn like_with_word_no_violation() {
    let diags = check("SELECT * FROM t WHERE name LIKE '%foo%'");
    assert!(diags.is_empty());
}

#[test]
fn like_with_underscore_no_violation() {
    let diags = check("SELECT * FROM t WHERE name LIKE '_'");
    assert!(diags.is_empty());
}

#[test]
fn like_empty_string_no_violation() {
    let diags = check("SELECT * FROM t WHERE name LIKE ''");
    assert!(diags.is_empty());
}

#[test]
fn ilike_percent_only_violation() {
    let diags = check("SELECT * FROM t WHERE name ILIKE '%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_like_percent_only_no_violation() {
    // NOT LIKE '%' matches nothing — do NOT flag it (it is the opposite of tautology)
    let diags = check("SELECT * FROM t WHERE name NOT LIKE '%'");
    assert!(diags.is_empty());
}

#[test]
fn two_like_tautologies_two_violations() {
    let sql = "SELECT * FROM t WHERE name LIKE '%' OR email LIKE '%%'";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_mentions_matches_everything_or_no_op() {
    let diags = check("SELECT * FROM t WHERE name LIKE '%'");
    let msg = &diags[0].message;
    assert!(
        msg.contains("matches everything") || msg.contains("no-op"),
        "message was: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM t WHERE name LIKE '%'");
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn select_with_no_like_no_violation() {
    let diags = check("SELECT name FROM t WHERE name = 'foo'");
    assert!(diags.is_empty());
}

#[test]
fn like_triple_percent_violation() {
    let diags = check("SELECT * FROM t WHERE name LIKE '%%%'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn like_percent_only_points_to_like_keyword() {
    // "SELECT * FROM t WHERE name LIKE '%'"
    //  123456789012345678901234567890123456
    //  LIKE starts at col 28
    let diags = check("SELECT * FROM t WHERE name LIKE '%'");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 28);
}

#[test]
fn ilike_percent_only_points_to_ilike_keyword() {
    // "SELECT * FROM t WHERE name ILIKE '%'"
    //  ILIKE starts at col 28
    let diags = check("SELECT * FROM t WHERE name ILIKE '%'");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 28);
}
