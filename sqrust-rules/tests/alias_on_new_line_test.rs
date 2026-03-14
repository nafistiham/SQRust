use sqrust_core::FileContext;
use sqrust_rules::layout::alias_on_new_line::AliasOnNewLine;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(AliasOnNewLine.name(), "Layout/AliasOnNewLine");
}

// ── No violation: alias on same line ──────────────────────────────────────────

#[test]
fn alias_same_line_no_violation() {
    let diags = AliasOnNewLine.check(&ctx("SELECT * FROM my_table AS t WHERE id = 1"));
    assert!(diags.is_empty());
}

#[test]
fn alias_same_line_no_as_keyword_no_violation() {
    let diags = AliasOnNewLine.check(&ctx("SELECT * FROM my_table t WHERE id = 1"));
    assert!(diags.is_empty());
}

#[test]
fn multiple_same_line_aliases_no_violation() {
    let sql = "SELECT *\nFROM a AS x\nJOIN b AS y ON x.id = y.id";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Violations: AS on its own line ────────────────────────────────────────────

#[test]
fn alias_on_new_line_violation() {
    let sql = "SELECT *\nFROM my_table\nAS t\nWHERE id = 1";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn alias_on_new_line_with_indentation_violation() {
    let sql = "SELECT *\nFROM my_table\n  AS t\nWHERE id = 1";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn join_alias_on_new_line_violation() {
    let sql = "SELECT *\nFROM a\nJOIN my_long_table_name\n  AS t ON a.id = t.id";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_violations_reported() {
    let sql = "SELECT *\nFROM table_one\n  AS t1\nJOIN table_two\n  AS t2 ON t1.id = t2.id";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 2);
}

// ── No violation: AS preceded by `)` — subquery or CTE ───────────────────────

#[test]
fn subquery_alias_on_new_line_no_violation() {
    let sql = "SELECT *\nFROM (\n  SELECT 1\n)\nAS sub";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn cte_as_paren_on_new_line_no_violation() {
    let sql = "WITH cte\nAS (\n  SELECT 1\n)\nSELECT * FROM cte";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── No violation: AS inside string literals ───────────────────────────────────

#[test]
fn as_in_string_no_violation() {
    let sql = "SELECT 'table\nAS alias' FROM t";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── No violation: AS inside comments ──────────────────────────────────────────

#[test]
fn as_in_line_comment_no_violation() {
    let sql = "SELECT * FROM t -- table\n-- AS alias\nWHERE id = 1";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn as_in_block_comment_no_violation() {
    let sql = "SELECT * FROM t /* table\nAS alias */ WHERE id = 1";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Edge cases ────────────────────────────────────────────────────────────────

#[test]
fn empty_file_no_violation() {
    let diags = AliasOnNewLine.check(&ctx(""));
    assert!(diags.is_empty());
}

#[test]
fn case_insensitive_as_keyword() {
    let sql = "SELECT *\nFROM my_table\nas t\nWHERE id = 1";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

// ── Message and position ──────────────────────────────────────────────────────

#[test]
fn violation_message_contains_alias_hint() {
    let sql = "SELECT *\nFROM my_table\nAS t\nWHERE id = 1";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("alias"),
        "message should mention 'alias', got: {}",
        diags[0].message
    );
}

#[test]
fn violation_line_and_col_are_nonzero() {
    let sql = "SELECT *\nFROM my_table\nAS t\nWHERE id = 1";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line > 0);
    assert!(diags[0].col > 0);
}

#[test]
fn violation_points_to_as_line() {
    // "AS t" starts on line 3
    let sql = "SELECT *\nFROM my_table\nAS t\nWHERE id = 1";
    let diags = AliasOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}
