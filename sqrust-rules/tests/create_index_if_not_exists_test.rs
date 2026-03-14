use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::create_index_if_not_exists::CreateIndexIfNotExists;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CreateIndexIfNotExists.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CreateIndexIfNotExists.name(), "Lint/CreateIndexIfNotExists");
}

#[test]
fn create_index_without_if_not_exists_violation() {
    let sql = "CREATE INDEX idx_name ON my_table (col)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_index_if_not_exists_no_violation() {
    let sql = "CREATE INDEX IF NOT EXISTS idx_name ON my_table (col)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_unique_index_violation() {
    let sql = "CREATE UNIQUE INDEX idx_unique ON my_table (col)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_unique_index_if_not_exists_no_violation() {
    let sql = "CREATE UNIQUE INDEX IF NOT EXISTS idx_unique ON my_table (col)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn parse_error_no_violations() {
    let sql = "NOT VALID SQL ###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = CreateIndexIfNotExists.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn two_create_indexes_two_violations() {
    let sql = "CREATE INDEX idx_a ON t (a);\nCREATE INDEX idx_b ON t (b)";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn mixed_indexes_one_violation() {
    let sql =
        "CREATE INDEX IF NOT EXISTS idx_safe ON t (a);\nCREATE INDEX idx_risky ON t (b)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_mentions_if_not_exists() {
    let sql = "CREATE INDEX idx_name ON my_table (col)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let lower = diags[0].message.to_lowercase();
    assert!(
        lower.contains("if not exists"),
        "message should mention IF NOT EXISTS: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "CREATE INDEX idx_name ON my_table (col)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn create_table_no_violation() {
    let sql = "CREATE TABLE t (id INT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_index_name_in_message() {
    let sql = "CREATE INDEX my_special_idx ON my_table (col)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("my_special_idx"),
        "message should mention the index name: {}",
        diags[0].message
    );
}

#[test]
fn create_index_with_where_clause_violation() {
    let sql = "CREATE INDEX idx ON t (col) WHERE col > 0";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
