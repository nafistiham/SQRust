use sqrust_core::FileContext;
use sqrust_rules::layout::space_after_as::SpaceAfterAs;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SpaceAfterAs.name(), "Layout/SpaceAfterAs");
}

// ── Basic violations ──────────────────────────────────────────────────────────

#[test]
fn as_alias_no_space_violation() {
    let diags = SpaceAfterAs.check(&ctx("SELECT col ASalias FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn as_alias_with_space_no_violation() {
    let diags = SpaceAfterAs.check(&ctx("SELECT col AS alias FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn as_in_cte_no_space_violation() {
    // CTE uses AS keyword with alias directly after (no space), e.g. the final SELECT part
    // WITH cte AS (SELECT 1) SELECT * FROM cte ASmy_alias
    let diags = SpaceAfterAs.check(&ctx("WITH cte AS (SELECT 1) SELECT * FROM cte ASmy_alias"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn as_in_cast_with_space_no_violation() {
    let diags = SpaceAfterAs.check(&ctx("SELECT CAST(x AS INT) FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn as_in_case_with_space_no_violation() {
    let diags = SpaceAfterAs.check(&ctx(
        "SELECT CASE WHEN x = 1 THEN 'a' END AS result FROM t",
    ));
    assert!(diags.is_empty());
}

#[test]
fn table_alias_no_space_violation() {
    let diags = SpaceAfterAs.check(&ctx(
        "SELECT * FROM t1 JOIN t2 ASnew_alias ON t1.id = t2.id",
    ));
    assert_eq!(diags.len(), 1);
}

#[test]
fn table_alias_with_space_no_violation() {
    let diags = SpaceAfterAs.check(&ctx("SELECT * FROM t AS alias"));
    assert!(diags.is_empty());
}

#[test]
fn multiple_violations() {
    let diags = SpaceAfterAs.check(&ctx("SELECT a ASx, b ASy FROM t"));
    assert_eq!(diags.len(), 2);
}

#[test]
fn as_in_string_no_violation() {
    let diags = SpaceAfterAs.check(&ctx("SELECT 'col ASalias' FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn as_in_comment_no_violation() {
    let diags = SpaceAfterAs.check(&ctx("-- SELECT col ASalias\nSELECT 1"));
    assert!(diags.is_empty());
}

#[test]
fn message_content() {
    let diags = SpaceAfterAs.check(&ctx("SELECT col ASalias FROM t"));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("AS"),
        "message should mention AS"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = SpaceAfterAs.check(&ctx("SELECT col ASalias FROM t"));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn as_in_database_word_no_violation() {
    // DATABASE contains AS internally but the char before A is B (alphanumeric), so no word boundary
    let diags = SpaceAfterAs.check(&ctx("SELECT DATABASE() FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn as_in_case_keyword_no_violation() {
    // CASE contains no standalone AS; CASE has C before A at index 1 — not word boundary
    let diags = SpaceAfterAs.check(&ctx("SELECT CASE WHEN 1=1 THEN 'a' END FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn lowercase_as_no_space_violation() {
    let diags = SpaceAfterAs.check(&ctx("SELECT col asalias FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn as_followed_by_newline_no_violation() {
    // AS followed by newline then alias — newline is not alphanumeric, so not flagged
    let diags = SpaceAfterAs.check(&ctx("SELECT col AS\n  alias FROM t"));
    assert!(diags.is_empty());
}
