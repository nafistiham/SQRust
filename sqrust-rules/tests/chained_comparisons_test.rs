use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::chained_comparisons::ChainedComparisons;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ChainedComparisons.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ChainedComparisons.name(), "Ambiguous/ChainedComparisons");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = ChainedComparisons.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn lt_lt_chain_one_violation() {
    let diags = check("SELECT * FROM t WHERE 1 < a AND a < 10");
    // 'AND' joined comparisons — no violation
    assert!(diags.is_empty());
}

#[test]
fn lt_lt_chain_actual_one_violation() {
    // True chained comparison: 1 < a < 10
    let diags = check("SELECT * FROM t WHERE 1 < a < 10");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/ChainedComparisons");
}

#[test]
fn gt_gt_chain_one_violation() {
    let diags = check("SELECT * FROM t WHERE a > b > c");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/ChainedComparisons");
}

#[test]
fn eq_eq_chain_one_violation() {
    let diags = check("SELECT * FROM t WHERE a = b = c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn lt_eq_chain_one_violation() {
    let diags = check("SELECT * FROM t WHERE a <= b <= c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn single_comparison_no_violation() {
    let diags = check("SELECT * FROM t WHERE a < b");
    assert!(diags.is_empty());
}

#[test]
fn and_joined_comparisons_no_violation() {
    let diags = check("SELECT * FROM t WHERE a < b AND b < c");
    assert!(diags.is_empty());
}

#[test]
fn arithmetic_binary_op_no_violation() {
    // a + b < c: left side of outer < is BinaryOp but with Add, not a comparison
    let diags = check("SELECT * FROM t WHERE a + b < c");
    assert!(diags.is_empty());
}

#[test]
fn two_chained_comparisons_two_violations() {
    // Two separate chained comparisons in one WHERE
    let diags = check("SELECT * FROM t WHERE (a < b < c) AND (x > y > z)");
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_mentions_chained_or_ambiguous() {
    let diags = check("SELECT * FROM t WHERE a < b < c");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message.to_lowercase();
    assert!(
        msg.contains("chained") || msg.contains("ambiguous"),
        "expected message to mention 'chained' or 'ambiguous', got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM t WHERE a < b < c");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line must be >= 1");
    assert!(diags[0].col >= 1, "col must be >= 1");
}

#[test]
fn nested_chain_flagged() {
    // a < b < c < d — at least one violation (the innermost chain)
    let diags = check("SELECT * FROM t WHERE a < b < c < d");
    assert!(!diags.is_empty(), "expected at least one violation for a < b < c < d");
}

#[test]
fn select_projection_chain_violation() {
    // Chain in SELECT item (a comparison expression in projection)
    let diags = check("SELECT a < b < c FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_eq_chain_one_violation() {
    let diags = check("SELECT * FROM t WHERE a != b != c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn gt_eq_chain_one_violation() {
    let diags = check("SELECT * FROM t WHERE a >= b >= c");
    assert_eq!(diags.len(), 1);
}
