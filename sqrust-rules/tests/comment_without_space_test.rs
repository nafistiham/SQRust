use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::comment_without_space::CommentWithoutSpace;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CommentWithoutSpace.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CommentWithoutSpace.name(), "Lint/CommentWithoutSpace");
}

#[test]
fn line_comment_no_space_violation() {
    // --no space immediately after --
    let sql = "SELECT 1 --no space";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn line_comment_with_space_no_violation() {
    let sql = "SELECT 1 -- has space";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn line_comment_triple_dash_no_violation() {
    // --- separator style should not be flagged
    let sql = "--- separator line\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn block_comment_no_space_violation() {
    let sql = "SELECT /*no space*/ 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn block_comment_with_space_no_violation() {
    let sql = "SELECT /* has space */ 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn line_comment_at_start_of_line_violation() {
    let sql = "--no space\nSELECT 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn line_comment_newline_no_violation() {
    // -- followed immediately by newline is an empty comment — valid
    let sql = "--\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiple_violations() {
    let sql = "--bad\nSELECT /*also bad*/ 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_line_comment_content() {
    let sql = "SELECT 1 --oops";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("--"),
        "expected message to mention --, got: {}",
        diags[0].message
    );
    assert!(
        diags[0].message.contains("space"),
        "expected message to mention space, got: {}",
        diags[0].message
    );
}

#[test]
fn message_block_comment_content() {
    let sql = "SELECT /*oops*/ 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("/*"),
        "expected message to mention /*, got: {}",
        diags[0].message
    );
    assert!(
        diags[0].message.contains("space"),
        "expected message to mention space, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "SELECT 1 --bad";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn block_comment_empty_no_violation() {
    // /**/ — /* immediately followed by */ should not be flagged
    let sql = "SELECT /**/ 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn line_comment_in_string_no_violation() {
    // '--no space' is a string literal, not a comment
    let sql = "SELECT '--no space' FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}
