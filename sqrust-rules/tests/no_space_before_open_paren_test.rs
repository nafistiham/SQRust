use sqrust_core::FileContext;
use sqrust_rules::layout::no_space_before_open_paren::NoSpaceBeforeOpenParen;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoSpaceBeforeOpenParen.name(), "Layout/NoSpaceBeforeOpenParen");
}

// ── No violation: function calls without tab ──────────────────────────────────

#[test]
fn function_no_space_before_paren_no_violation() {
    let diags = NoSpaceBeforeOpenParen.check(&ctx("SELECT COUNT(*) FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn function_single_space_before_paren_no_violation() {
    // Single space is already covered by FunctionCallSpacing — we only flag tabs.
    let diags = NoSpaceBeforeOpenParen.check(&ctx("SELECT COUNT(*) FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn coalesce_no_space_no_violation() {
    let diags = NoSpaceBeforeOpenParen.check(&ctx("SELECT COALESCE(a, b) FROM t"));
    assert!(diags.is_empty());
}

// ── Violations: tab before ( in function calls ────────────────────────────────

#[test]
fn tab_before_paren_in_count_violation() {
    let sql = "SELECT COUNT\t(*) FROM t";
    let diags = NoSpaceBeforeOpenParen.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn tab_before_paren_in_coalesce_violation() {
    let sql = "SELECT COALESCE\t(a, b) FROM t";
    let diags = NoSpaceBeforeOpenParen.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn tab_before_paren_lowercase_function_violation() {
    let sql = "SELECT sum\t(x) FROM t";
    let diags = NoSpaceBeforeOpenParen.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_tab_violations_reported() {
    let sql = "SELECT COUNT\t(*), SUM\t(x) FROM t";
    let diags = NoSpaceBeforeOpenParen.check(&ctx(sql));
    assert_eq!(diags.len(), 2);
}

// ── No violation: keywords with tab before ( ─────────────────────────────────

#[test]
fn in_keyword_tab_before_paren_no_violation() {
    // SQL keywords like IN are not function calls.
    let sql = "SELECT * FROM t WHERE col IN\t(1, 2)";
    let diags = NoSpaceBeforeOpenParen.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn not_keyword_tab_before_paren_no_violation() {
    let sql = "SELECT * FROM t WHERE NOT\t(a = 1)";
    let diags = NoSpaceBeforeOpenParen.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn exists_keyword_tab_before_paren_no_violation() {
    let sql = "SELECT * FROM t WHERE EXISTS\t(SELECT 1 FROM s)";
    let diags = NoSpaceBeforeOpenParen.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── No violation: tab before ( inside string literals ────────────────────────

#[test]
fn tab_before_paren_in_string_no_violation() {
    let sql = "SELECT 'COUNT\t(*)' FROM t";
    let diags = NoSpaceBeforeOpenParen.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── No violation: tab before ( inside comments ────────────────────────────────

#[test]
fn tab_before_paren_in_line_comment_no_violation() {
    let sql = "SELECT a FROM t -- COUNT\t(*) is fine here";
    let diags = NoSpaceBeforeOpenParen.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn tab_before_paren_in_block_comment_no_violation() {
    let sql = "SELECT a FROM t /* COUNT\t(*) */ WHERE id = 1";
    let diags = NoSpaceBeforeOpenParen.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Edge cases ────────────────────────────────────────────────────────────────

#[test]
fn empty_file_no_violation() {
    let diags = NoSpaceBeforeOpenParen.check(&ctx(""));
    assert!(diags.is_empty());
}

// ── Message and position ──────────────────────────────────────────────────────

#[test]
fn violation_message_contains_function_name() {
    let sql = "SELECT COUNT\t(*) FROM t";
    let diags = NoSpaceBeforeOpenParen.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("COUNT"),
        "message should contain function name, got: {}",
        diags[0].message
    );
}

#[test]
fn violation_message_contains_tab_hint() {
    let sql = "SELECT COUNT\t(*) FROM t";
    let diags = NoSpaceBeforeOpenParen.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("tab") || diags[0].message.contains("'('"),
        "message should mention tab or '(', got: {}",
        diags[0].message
    );
}

#[test]
fn violation_line_and_col_are_nonzero() {
    let sql = "SELECT COUNT\t(*) FROM t";
    let diags = NoSpaceBeforeOpenParen.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line > 0);
    assert!(diags[0].col > 0);
}
