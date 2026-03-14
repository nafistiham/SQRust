use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::excessive_union_chain::ExcessiveUnionChain;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ExcessiveUnionChain.check(&ctx)
}

/// Build a UNION ALL chain with `n` SELECT arms (produces n-1 set operators).
fn make_union_all(n: usize) -> String {
    let arms: Vec<String> = (1..=n).map(|i| format!("SELECT {i}")).collect();
    arms.join(" UNION ALL ")
}

/// Build a UNION chain with `n` SELECT arms.
fn make_union(n: usize) -> String {
    let arms: Vec<String> = (1..=n).map(|i| format!("SELECT {i}")).collect();
    arms.join(" UNION ")
}

/// Build an INTERSECT chain with `n` SELECT arms.
fn make_intersect(n: usize) -> String {
    let arms: Vec<String> = (1..=n).map(|i| format!("SELECT {i}")).collect();
    arms.join(" INTERSECT ")
}

/// Build an EXCEPT chain with `n` SELECT arms.
fn make_except(n: usize) -> String {
    let arms: Vec<String> = (1..=n).map(|i| format!("SELECT {i}")).collect();
    arms.join(" EXCEPT ")
}

// ── rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(ExcessiveUnionChain.name(), "Structure/ExcessiveUnionChain");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = ExcessiveUnionChain.check(&ctx);
        assert!(diags.is_empty());
    }
}

// ── below threshold → no violation ────────────────────────────────────────────

#[test]
fn four_set_ops_no_violation() {
    // 5 arms = 4 UNION ALLs, which is below the threshold of 5
    let diags = check(&make_union_all(5));
    assert!(diags.is_empty(), "4 set ops should not be flagged");
}

#[test]
fn three_set_ops_no_violation() {
    // 4 arms = 3 UNIONs
    let diags = check(&make_union(4));
    assert!(diags.is_empty());
}

#[test]
fn single_select_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_set_ops_no_violation() {
    let diags = check("SELECT id, name FROM orders WHERE status = 'active'");
    assert!(diags.is_empty());
}

// ── at threshold: 5 set ops → 1 violation ─────────────────────────────────────

#[test]
fn five_union_all_one_violation() {
    // 6 arms = 5 UNION ALLs — at threshold
    let diags = check(&make_union_all(6));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/ExcessiveUnionChain");
}

#[test]
fn five_union_one_violation() {
    // 6 arms = 5 UNIONs — at threshold
    let diags = check(&make_union(6));
    assert_eq!(diags.len(), 1);
}

#[test]
fn five_intersect_one_violation() {
    // 6 arms = 5 INTERSECTs — at threshold
    let diags = check(&make_intersect(6));
    assert_eq!(diags.len(), 1);
}

#[test]
fn five_except_one_violation() {
    // 6 arms = 5 EXCEPTs — at threshold
    let diags = check(&make_except(6));
    assert_eq!(diags.len(), 1);
}

// ── above threshold ────────────────────────────────────────────────────────────

#[test]
fn ten_union_all_one_violation() {
    // 11 arms = 10 UNION ALLs
    let diags = check(&make_union_all(11));
    assert_eq!(diags.len(), 1);
}

// ── message content ────────────────────────────────────────────────────────────

#[test]
fn message_contains_count() {
    // 6 arms = 5 set ops
    let diags = check(&make_union_all(6));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('5'),
        "message should contain the count (5), got: {}",
        diags[0].message
    );
}

#[test]
fn message_contains_cte_or_refactor_hint() {
    let diags = check(&make_union_all(6));
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("cte") || msg.contains("refactor") || msg.contains("derived") || msg.contains("maintainability"),
        "expected message to mention refactoring, got: {}",
        diags[0].message
    );
}

// ── diagnostic position ────────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check(&make_union_all(6));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line must be >= 1");
    assert!(diags[0].col >= 1, "col must be >= 1");
}

// ── mixed operators ────────────────────────────────────────────────────────────

#[test]
fn mixed_union_intersect_except_counts_toward_threshold() {
    // 3 UNION + 1 INTERSECT + 1 EXCEPT = 5 total → flag
    let sql = "SELECT 1 UNION SELECT 2 UNION SELECT 3 UNION SELECT 4 INTERSECT SELECT 5 EXCEPT SELECT 6";
    // This may not parse cleanly depending on precedence, so handle parse failure gracefully
    let ctx = FileContext::from_source(sql, "test.sql");
    if ctx.parse_errors.is_empty() {
        let diags = ExcessiveUnionChain.check(&ctx);
        assert_eq!(diags.len(), 1, "5 mixed set ops should be flagged");
    }
    // If parser rejects it, the test still passes (no crash)
}
