SELECT 'tokens' AS entity, count(*) AS rows, coalesce(max(entity_index), 0) AS max_index FROM tokens
UNION ALL SELECT 'token_trades', count(*), coalesce(max(entity_index), 0) FROM token_trade_events
UNION ALL SELECT 'token_transfers', count(*), coalesce(max(entity_index), 0) FROM token_transfer_events
UNION ALL SELECT 'listed_tokens', count(*), coalesce(max(entity_index), 0) FROM token_listings
UNION ALL SELECT 'ipshare_trades', count(*), coalesce(max(entity_index), 0) FROM ipshare_trade_events
UNION ALL SELECT 'value_captures', count(*), coalesce(max(entity_index), 0) FROM ipshare_value_capture_events
UNION ALL SELECT 'stakes', count(*), coalesce(max(entity_index), 0) FROM ipshare_stake_events
UNION ALL SELECT 'walnut_communities', count(*), coalesce(max(entity_index), 0) FROM walnut_communities
UNION ALL SELECT 'walnut_pools', count(*), coalesce(max(entity_index), 0) FROM walnut_pools
UNION ALL SELECT 'walnut_operations', count(*), coalesce(max(entity_index), 0) FROM walnut_operations
ORDER BY entity;

SELECT * FROM pump_summary;
SELECT * FROM ipshare_summary;
SELECT * FROM walnut_summary;

SELECT entity_index, operation_type, community, pool_factory, pool, account, asset, amount
FROM walnut_operations
ORDER BY entity_index;
