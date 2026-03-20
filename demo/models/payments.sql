SELECT
    id,
    order_id,
    CAST(amount AS NUMERIC) AS amount,
    payment_method,
    created_at
FROM raw.payments
WHERE status <> 'failed';
