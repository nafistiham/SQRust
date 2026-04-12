/// End-to-end integration tests: BigQuery dialect parses real dbt/BigQuery SQL
/// without producing Parse/Error diagnostics.
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn check_no_parse_errors(sql: &str, dialect: &str) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("q.sql");
    fs::write(&path, sql).unwrap();

    let output = Command::cargo_bin("sqrust")
        .unwrap()
        .args(["check", "--dialect", dialect, path.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Parse/Error"),
        "dialect={dialect}: unexpected parse error:\n{stdout}\nSQL:\n{sql}"
    );
    // Exit code 0 (no violations) or 1 (lint violations) are both fine.
    // Exit code 2 is a tool error.
    assert_ne!(
        output.status.code(),
        Some(2),
        "dialect={dialect}: tool exited with code 2 (tool error), stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ─── Backtick identifiers ─────────────────────────────────────────────────

#[test]
fn bigquery_backtick_project_dataset_table() {
    check_no_parse_errors(
        "SELECT id FROM `my_project.my_dataset.orders`;\n",
        "bigquery",
    );
}

#[test]
fn bigquery_backtick_column_names() {
    check_no_parse_errors(
        "SELECT `order_id`, `customer_id`, `amount` FROM `sales.orders`;\n",
        "bigquery",
    );
}

// ─── BigQuery types ───────────────────────────────────────────────────────

#[test]
fn bigquery_int64_type() {
    check_no_parse_errors(
        "SELECT CAST(revenue AS INT64) AS revenue_int FROM orders;\n",
        "bigquery",
    );
}

#[test]
fn bigquery_float64_type() {
    check_no_parse_errors(
        "SELECT CAST(price AS FLOAT64) AS price_f FROM products;\n",
        "bigquery",
    );
}

// ─── SAFE_CAST / SAFE_DIVIDE ──────────────────────────────────────────────

#[test]
fn bigquery_safe_cast() {
    check_no_parse_errors(
        "SELECT SAFE_CAST(raw_date AS DATE) AS order_date FROM events;\n",
        "bigquery",
    );
}

// ─── ARRAY / STRUCT ───────────────────────────────────────────────────────

#[test]
fn bigquery_array_agg() {
    check_no_parse_errors(
        "SELECT customer_id, ARRAY_AGG(order_id) AS orders FROM sales GROUP BY customer_id;\n",
        "bigquery",
    );
}

// ─── DATE functions ───────────────────────────────────────────────────────

#[test]
fn bigquery_date_trunc() {
    check_no_parse_errors(
        "SELECT DATE_TRUNC(order_date, MONTH) AS month FROM orders;\n",
        "bigquery",
    );
}

#[test]
fn bigquery_date_diff() {
    check_no_parse_errors(
        "SELECT DATE_DIFF(CURRENT_DATE(), order_date, DAY) AS days_since FROM orders;\n",
        "bigquery",
    );
}

// ─── Typical dbt model ────────────────────────────────────────────────────

#[test]
fn bigquery_typical_dbt_model() {
    check_no_parse_errors(
        r#"WITH base AS (
    SELECT
        order_id,
        customer_id,
        SAFE_CAST(order_date AS DATE) AS order_date,
        CAST(amount AS FLOAT64) AS amount
    FROM `analytics.raw.orders`
    WHERE order_date >= '2024-01-01'
),
enriched AS (
    SELECT
        b.order_id,
        b.customer_id,
        b.order_date,
        b.amount,
        DATE_TRUNC(b.order_date, MONTH) AS order_month
    FROM base AS b
)
SELECT * FROM enriched;
"#,
        "bigquery",
    );
}
