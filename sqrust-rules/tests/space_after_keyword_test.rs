use sqrust_core::FileContext;
use sqrust_rules::layout::space_after_keyword::SpaceAfterKeyword;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SpaceAfterKeyword.name(), "Layout/SpaceAfterKeyword");
}

// ── Basic violations ──────────────────────────────────────────────────────────

#[test]
fn where_no_space_violation() {
    let diags = SpaceAfterKeyword.check(&ctx("SELECT * FROM t WHERE(x = 1)"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn where_with_space_no_violation() {
    let diags = SpaceAfterKeyword.check(&ctx("SELECT * FROM t WHERE (x = 1)"));
    assert!(diags.is_empty());
}

#[test]
fn and_no_space_violation() {
    let diags = SpaceAfterKeyword.check(&ctx("SELECT * FROM t WHERE a = 1 AND(b = 2)"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn or_no_space_violation() {
    let diags = SpaceAfterKeyword.check(&ctx("SELECT * FROM t WHERE a = 1 OR(b = 2)"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_no_space_violation() {
    let diags = SpaceAfterKeyword.check(&ctx("SELECT * FROM t WHERE NOT(x = 1)"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn in_no_space_violation() {
    let diags = SpaceAfterKeyword.check(&ctx("SELECT * FROM t WHERE x IN(1, 2)"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn in_with_space_no_violation() {
    let diags = SpaceAfterKeyword.check(&ctx("SELECT * FROM t WHERE x IN (1, 2)"));
    assert!(diags.is_empty());
}

#[test]
fn function_call_no_violation() {
    let diags = SpaceAfterKeyword.check(&ctx("SELECT COALESCE(a, b) FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn having_no_space_violation() {
    let diags =
        SpaceAfterKeyword.check(&ctx("SELECT x FROM t GROUP BY x HAVING(COUNT(*) > 1)"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_violations_count() {
    let diags = SpaceAfterKeyword.check(&ctx("SELECT * FROM t WHERE(a=1) AND(b=2)"));
    assert_eq!(diags.len(), 2);
}

#[test]
fn parse_error_still_scans() {
    // Source-level scan works even when SQL is not parseable.
    let diags = SpaceAfterKeyword.check(&ctx("SELECT col FROM FROM WHERE(a = 1)"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_content() {
    let diags = SpaceAfterKeyword.check(&ctx("SELECT * FROM t WHERE(a = 1)"));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("WHERE"),
        "message should mention the keyword"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = SpaceAfterKeyword.check(&ctx("SELECT * FROM t WHERE(a = 1)"));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn exists_no_space_violation() {
    let diags =
        SpaceAfterKeyword.check(&ctx("SELECT * FROM t WHERE EXISTS(SELECT 1 FROM s)"));
    assert_eq!(diags.len(), 1);
}
