WITH orders AS (
    SELECT * FROM raw.orders
),

order_items AS (
    SELECT * FROM raw.order_items
),

unused_cte AS (
    SELECT id FROM raw.other_table
),

final AS (
    SELECT
        o.id,
        o.customer_id,
        o.status,
        SUM(oi.amount) AS total_amount
    FROM orders AS o
    INNER JOIN order_items AS oi ON o.id = oi.order_id
    INNER JOIN order_items AS oi2 ON o.id = oi2.order_id
    GROUP BY o.id, o.customer_id, o.status
)

SELECT * FROM final;
