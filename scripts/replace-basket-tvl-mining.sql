-- Remove every indexed row produced by the retired Basket TVL mining protocol.
-- Run only while the Substreams sink and the MySQL sync worker are stopped.
-- The NFT mining tables and all non-Basket Nutbox data are intentionally preserved.

BEGIN;

CREATE TEMP TABLE retired_basket_tvl_pools ON COMMIT DROP AS
SELECT id
FROM walnut_pools
WHERE pool_type = 'BASKET_TVL_MINING';

DELETE FROM walnut_account_pools
WHERE pool IN (SELECT id FROM retired_basket_tvl_pools);

DELETE FROM walnut_pool_stakers
WHERE pool IN (SELECT id FROM retired_basket_tvl_pools);

DELETE FROM walnut_basket_child_positions;
DELETE FROM walnut_basket_child_events;
DELETE FROM walnut_basket_child_pools;
DELETE FROM walnut_basket_tvl_events;
DELETE FROM walnut_basket_stakes;
DELETE FROM walnut_basket_tvl_pools;

DELETE FROM walnut_pools
WHERE id IN (SELECT id FROM retired_basket_tvl_pools);

UPDATE walnut_communities AS community
SET pools_count = counts.total_pools,
    active_pool_count = counts.active_pools
FROM (
    SELECT
        community.id,
        COUNT(pool.id)::bigint AS total_pools,
        COUNT(pool.id) FILTER (WHERE pool.status = 'OPENED')::bigint AS active_pools
    FROM walnut_communities AS community
    LEFT JOIN walnut_pools AS pool ON pool.community = community.id
    GROUP BY community.id
) AS counts
WHERE community.id = counts.id;

UPDATE walnut_summary
SET total_pools = (SELECT COUNT(*)::bigint FROM walnut_pools);

COMMIT;

