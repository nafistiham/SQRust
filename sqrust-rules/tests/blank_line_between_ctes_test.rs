use sqrust_core::FileContext;
use sqrust_rules::layout::blank_line_between_ctes::BlankLineBetweenCTEs;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    BlankLineBetweenCTEs.check(&FileContext::from_source(sql, "test.sql"))
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(BlankLineBetweenCTEs.name(), "Layout/BlankLineBetweenCTEs");
}

// ── Basic violations ──────────────────────────────────────────────────────────

#[test]
fn two_ctes_no_blank_line_violation() {
    let sql = "WITH\n  cte1 AS (\n    SELECT 1 AS a\n  ),\n  cte2 AS (\n    SELECT 2 AS b\n  )\nSELECT * FROM cte1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_ctes_with_blank_line_no_violation() {
    let sql = "WITH\n  cte1 AS (\n    SELECT 1 AS a\n  ),\n\n  cte2 AS (\n    SELECT 2 AS b\n  )\nSELECT * FROM cte1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn single_cte_no_violation() {
    let sql = "WITH cte1 AS (\n  SELECT 1\n)\nSELECT * FROM cte1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn no_cte_no_violation() {
    let sql = "SELECT id FROM t WHERE id > 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn three_ctes_two_missing_blank_lines_two_violations() {
    let sql = "WITH\n  a AS (\n    SELECT 1\n  ),\n  b AS (\n    SELECT 2\n  ),\n  c AS (\n    SELECT 3\n  )\nSELECT * FROM a";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn three_ctes_first_missing_second_ok_one_violation() {
    let sql = "WITH\n  a AS (\n    SELECT 1\n  ),\n  b AS (\n    SELECT 2\n  ),\n\n  c AS (\n    SELECT 3\n  )\nSELECT * FROM a";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_ctes_all_with_blank_lines_no_violation() {
    let sql = "WITH\n  a AS (\n    SELECT 1\n  ),\n\n  b AS (\n    SELECT 2\n  ),\n\n  c AS (\n    SELECT 3\n  )\nSELECT * FROM a";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn parse_error_still_scans() {
    // Source-level scan works even on invalid SQL
    let sql = "WITH a AS (\n  SELECT 1\n),\nb AS (\n  SELECT 2\n) FROM FROM";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn line_col_nonzero() {
    let sql = "WITH\n  cte1 AS (\n    SELECT 1\n  ),\n  cte2 AS (\n    SELECT 2\n  )\nSELECT * FROM cte1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn cte_with_comma_in_select_no_false_positive() {
    // Commas inside SELECT inside CTE should not trigger false positives
    let sql = "WITH\n  cte1 AS (\n    SELECT a, b, c FROM t\n  ),\n\n  cte2 AS (\n    SELECT x, y FROM s\n  )\nSELECT * FROM cte1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiline_cte_no_blank_line_violation() {
    let sql = concat!(
        "WITH\n",
        "  orders AS (\n",
        "    SELECT\n",
        "      order_id,\n",
        "      customer_id,\n",
        "      total\n",
        "    FROM raw_orders\n",
        "    WHERE status = 'complete'\n",
        "  ),\n",
        "  customers AS (\n",
        "    SELECT\n",
        "      customer_id,\n",
        "      name\n",
        "    FROM raw_customers\n",
        "  )\n",
        "SELECT * FROM orders"
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn cte_compact_style_violation() {
    // CTEs on consecutive lines without blank line
    let sql = "WITH a AS (SELECT 1),\nb AS (SELECT 2)\nSELECT * FROM a JOIN b ON 1=1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn inline_ctes_violation() {
    // Both CTEs on same line
    let sql = "WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a JOIN b ON 1=1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_mentions_blank_line_or_cte() {
    let sql = "WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("blank") || msg.contains("cte"),
        "expected message to mention blank line or CTE, got: {}",
        diags[0].message
    );
}

#[test]
fn cte_with_nested_parens_no_false_positive() {
    // CTE body has nested parens — depth tracking must not confuse the closing paren
    let sql = "WITH\n  a AS (\n    SELECT COALESCE(x, 1) FROM t\n  ),\n\n  b AS (\n    SELECT 2\n  )\nSELECT * FROM a";
    let diags = check(sql);
    assert!(diags.is_empty());
}
