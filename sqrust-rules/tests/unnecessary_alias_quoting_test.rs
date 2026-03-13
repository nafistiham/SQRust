use sqrust_core::FileContext;
use sqrust_rules::layout::unnecessary_alias_quoting::UnnecessaryAliasQuoting;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnnecessaryAliasQuoting.name(), "Layout/UnnecessaryAliasQuoting");
}

// ── Violations (unnecessary quoting) ─────────────────────────────────────────

#[test]
fn double_quoted_simple_name_produces_one_violation() {
    let diags = UnnecessaryAliasQuoting.check(&ctx(r#"SELECT col AS "simple_name" FROM t"#));
    assert_eq!(diags.len(), 1);
}

#[test]
fn backtick_simple_name_produces_one_violation() {
    let diags = UnnecessaryAliasQuoting.check(&ctx("SELECT col AS `backtick_name` FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn double_quoted_my_column_produces_one_violation() {
    let diags = UnnecessaryAliasQuoting.check(&ctx(r#"SELECT col AS "my_column" FROM t"#));
    assert_eq!(diags.len(), 1);
}

#[test]
fn double_quoted_upper_name_produces_one_violation() {
    // All-caps but simple identifier — still unnecessary
    let diags = UnnecessaryAliasQuoting.check(&ctx(r#"SELECT col AS "UPPER_NAME" FROM t"#));
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_unnecessary_quotes_produces_correct_count() {
    let diags = UnnecessaryAliasQuoting.check(
        &ctx(r#"SELECT col AS "first_col", col2 AS "second_col" FROM t"#),
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn subquery_unnecessary_quote_produces_one_violation() {
    let diags = UnnecessaryAliasQuoting.check(
        &ctx(r#"SELECT a FROM (SELECT col AS "renamed" FROM t) sub"#),
    );
    assert_eq!(diags.len(), 1);
}

// ── No violations (quoting IS needed or already unquoted) ─────────────────────

#[test]
fn unquoted_simple_name_produces_no_violations() {
    let diags = UnnecessaryAliasQuoting.check(&ctx("SELECT col AS simple_name FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn quoted_alias_with_space_produces_no_violations() {
    let diags = UnnecessaryAliasQuoting.check(&ctx(r#"SELECT col AS "my alias" FROM t"#));
    assert!(diags.is_empty());
}

#[test]
fn quoted_alias_starting_with_digit_produces_no_violations() {
    let diags = UnnecessaryAliasQuoting.check(&ctx(r#"SELECT col AS "123invalid" FROM t"#));
    assert!(diags.is_empty());
}

#[test]
fn quoted_alias_with_hyphen_produces_no_violations() {
    let diags = UnnecessaryAliasQuoting.check(&ctx(r#"SELECT col AS "col-name" FROM t"#));
    assert!(diags.is_empty());
}

#[test]
fn quoted_reserved_keyword_date_produces_no_violations() {
    let diags = UnnecessaryAliasQuoting.check(&ctx(r#"SELECT col AS "date" FROM t"#));
    assert!(diags.is_empty());
}

#[test]
fn quoted_reserved_keyword_select_produces_no_violations() {
    let diags = UnnecessaryAliasQuoting.check(&ctx(r#"SELECT col AS "select" FROM t"#));
    assert!(diags.is_empty());
}

// ── Parse errors ──────────────────────────────────────────────────────────────

#[test]
fn parse_error_produces_zero_violations() {
    // Broken SQL with a parse error — rule should gracefully return empty
    let sql = r#"SELECTT INVALID GARBAGE @@## AS "foo""#;
    let file_ctx = FileContext::from_source(sql, "test.sql");
    if !file_ctx.parse_errors.is_empty() {
        let diags = UnnecessaryAliasQuoting.check(&file_ctx);
        assert!(diags.is_empty(), "expected 0 violations on parse error, got {}", diags.len());
    }
    // If the parser accepted it, we simply don't assert (no panic = pass).
}

// ── Message content ───────────────────────────────────────────────────────────

#[test]
fn message_mentions_alias_name() {
    let diags = UnnecessaryAliasQuoting.check(&ctx(r#"SELECT col AS "simple_name" FROM t"#));
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("simple_name"),
        "message was: {msg}"
    );
}
