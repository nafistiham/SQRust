use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::non_deterministic_function::NonDeterministicFunction;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NonDeterministicFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        NonDeterministicFunction.name(),
        "Lint/NonDeterministicFunction"
    );
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "NOT VALID SQL ###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = NonDeterministicFunction.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn rand_function_violation() {
    let sql = "SELECT RAND()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn random_function_violation() {
    let sql = "SELECT RANDOM()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn uuid_function_violation() {
    let sql = "SELECT UUID()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn newid_function_violation() {
    let sql = "SELECT NEWID()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn gen_random_uuid_violation() {
    let sql = "SELECT GEN_RANDOM_UUID()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn deterministic_function_no_violation() {
    // ABS, COUNT, SUM are all deterministic — should not be flagged
    let sql = "SELECT ABS(-1), COUNT(*), SUM(amount) FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn rand_in_where_violation() {
    let sql = "SELECT id FROM t WHERE score > RAND()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn rand_in_select_violation() {
    let sql = "SELECT RAND() AS r FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_rand_calls_two_violations() {
    let sql = "SELECT RAND(), RAND() FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_contains_function_name() {
    let sql = "SELECT RAND()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("RAND"),
        "message should contain the function name RAND: {}",
        msg
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "SELECT RAND()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn lowercase_rand_violation() {
    // Function names are case-insensitive in SQL
    let sql = "SELECT rand()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
