-- Prepare PostgreSQL for the V0.5.2 filter-fix cutover at block 53,869,281.
--
-- Run only while the Substreams sink and downstream graph-data-sync are
-- stopped. The reset-to-cutover package is valid only while all checked state
-- tables are empty. The three already-backfilled IndexBroker events are retained,
-- but their sequential indexes are moved into the deterministic block/log
-- range so a fresh post-cutover counter cannot collide with them.

BEGIN;

DO $$
DECLARE
    nonempty_tables TEXT;
BEGIN
    SELECT string_agg(table_name, ', ' ORDER BY table_name)
    INTO nonempty_tables
    FROM (
        SELECT 'imported_token_trade_events' AS table_name
        WHERE EXISTS (SELECT 1 FROM imported_token_trade_events)
        UNION ALL
        SELECT 'v11_imported_markets'
        WHERE EXISTS (SELECT 1 FROM v11_imported_markets)
        UNION ALL
        SELECT 'walnut_index_broker_nft_pools'
        WHERE EXISTS (SELECT 1 FROM walnut_index_broker_nft_pools)
        UNION ALL
        SELECT 'walnut_index_broker_nft_amms'
        WHERE EXISTS (SELECT 1 FROM walnut_index_broker_nft_amms)
        UNION ALL
        SELECT 'walnut_index_broker_nft_tokens'
        WHERE EXISTS (SELECT 1 FROM walnut_index_broker_nft_tokens)
        UNION ALL
        SELECT 'walnut_index_broker_nft_accounts'
        WHERE EXISTS (SELECT 1 FROM walnut_index_broker_nft_accounts)
    ) AS nonempty;

    IF nonempty_tables IS NOT NULL THEN
        RAISE EXCEPTION
            'cutover reset is unsafe because these historical state tables are not empty: %',
            nonempty_tables;
    END IF;
END $$;

UPDATE walnut_index_broker_nft_events
SET entity_index = block_number * 1000000 + log_index;

COMMIT;

SELECT id, entity_index, block_number, log_index
FROM walnut_index_broker_nft_events
ORDER BY block_number, log_index, id;
