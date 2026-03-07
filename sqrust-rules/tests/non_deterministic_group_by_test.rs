use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::non_deterministic_group_by::NonDeterministicGroupBy;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NonDeterministicGroupBy.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        NonDeterministicGroupBy.name(),
        "Ambiguous/NonDeterministicGroupBy"
    );
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = NonDeterministicGroupBy.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn no_group_by_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

#[test]
fn normal_group_by_no_violation() {
    let diags = check("SELECT dept, COUNT(*) FROM t GROUP BY dept");
    assert!(diags.is_empty());
}

#[test]
fn rand_in_group_by_one_violation() {
    let diags = check("SELECT id FROM t GROUP BY RAND()");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/NonDeterministicGroupBy");
}

#[test]
fn random_in_group_by_one_violation() {
    let diags = check("SELECT id FROM t GROUP BY RANDOM()");
    assert_eq!(diags.len(), 1);
}

#[test]
fn uuid_in_group_by_one_violation() {
    let diags = check("SELECT id FROM t GROUP BY UUID()");
    assert_eq!(diags.len(), 1);
}

#[test]
fn deterministic_function_no_violation() {
    let diags = check("SELECT UPPER(name) FROM t GROUP BY UPPER(name)");
    assert!(diags.is_empty());
}

#[test]
fn rand_in_where_not_flagged() {
    let diags = check("SELECT id FROM t WHERE RAND() > 0.5");
    assert!(diags.is_empty());
}

#[test]
fn rand_in_select_not_flagged() {
    let diags = check("SELECT RAND() FROM t");
    assert!(diags.is_empty());
}

#[test]
fn two_rand_in_group_by_two_violations() {
    let diags = check("SELECT id FROM t GROUP BY RAND(), RAND()");
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_mentions_non_deterministic() {
    let diags = check("SELECT id FROM t GROUP BY RAND()");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("non-deterministic") || msg.contains("nondeterministic") || msg.contains("unpredictable"),
        "expected message to mention non-deterministic behaviour, got: {}",
        diags[0].message
    );
}

#[test]
fn line_nonzero() {
    let diags = check("SELECT id FROM t GROUP BY RAND()");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line must be >= 1");
}

#[test]
fn col_nonzero() {
    let diags = check("SELECT id FROM t GROUP BY RAND()");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1, "col must be >= 1");
}
