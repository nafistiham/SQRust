use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::function_on_filtered_column::FunctionOnFilteredColumn;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    FunctionOnFilteredColumn.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        FunctionOnFilteredColumn.name(),
        "Ambiguous/FunctionOnFilteredColumn"
    );
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = FunctionOnFilteredColumn.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn no_function_in_where_no_violation() {
    let diags = check("SELECT * FROM t WHERE name = 'foo'");
    assert!(diags.is_empty());
}

#[test]
fn function_on_column_eq_flagged() {
    let diags = check("SELECT * FROM t WHERE UPPER(name) = 'FOO'");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/FunctionOnFilteredColumn");
}

#[test]
fn function_on_column_gt_flagged() {
    let diags = check("SELECT * FROM t WHERE YEAR(created_at) > 2020");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/FunctionOnFilteredColumn");
}

#[test]
fn function_on_literal_not_flagged() {
    // Function on right side with a literal arg — should not flag
    let diags = check("SELECT * FROM t WHERE id = ABS(-1)");
    assert!(diags.is_empty());
}

#[test]
fn function_with_two_args_not_flagged() {
    // Multi-arg function — should not flag even if first arg is a column
    let diags = check("SELECT * FROM t WHERE SUBSTR(name, 1, 3) = 'foo'");
    assert!(diags.is_empty());
}

#[test]
fn function_on_expression_not_flagged() {
    // Arg is BinaryOp, not a bare column
    let diags = check("SELECT * FROM t WHERE ABS(a + b) = 5");
    assert!(diags.is_empty());
}

#[test]
fn join_on_function_flagged() {
    // Both sides of the JOIN ON use a function on a column
    let diags = check(
        "SELECT * FROM t1 JOIN t2 ON LOWER(t1.col) = LOWER(t2.col)",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn nested_function_not_flagged() {
    // UPPER(LOWER(name)) — inner arg is a Function, not a bare column
    let diags = check("SELECT * FROM t WHERE UPPER(LOWER(name)) = 'FOO'");
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_index() {
    let diags = check("SELECT * FROM t WHERE UPPER(name) = 'FOO'");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("index"),
        "expected message to mention 'index', got: {}",
        diags[0].message
    );
}

#[test]
fn line_nonzero() {
    let diags = check("SELECT * FROM t WHERE UPPER(name) = 'FOO'");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

#[test]
fn col_nonzero() {
    let diags = check("SELECT * FROM t WHERE UPPER(name) = 'FOO'");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn two_functions_in_where_two_violations() {
    let diags = check("SELECT * FROM t WHERE UPPER(a) = 'X' AND YEAR(b) = 2020");
    assert_eq!(diags.len(), 2);
}
