\set ON_ERROR_STOP on
-- Read-only acceptance queries; run after the domain backfill reaches its
-- recorded boundary. Numbers describe initialization, later admin edits may differ.
BEGIN READ ONLY;
SELECT event_type, count(*), min(block_number), max(block_number)
FROM v14_events GROUP BY event_type ORDER BY event_type;
SELECT approved, count(*) FROM pump_constituents
WHERE pump = '0xd72826378cb53182319f7a8b2882de4997ffc896' GROUP BY approved;
SELECT active, count(*) FROM v14_router_price_pools
WHERE router = '0xfc82178523687edd56f7474d6529a14f7655ab15' GROUP BY active;
SELECT active, count(*) FROM v14_router_routes
WHERE router = '0xfc82178523687edd56f7474d6529a14f7655ab15' GROUP BY active;
SELECT t.id, t.version, t.listing_status, t.bonding_curve_supply,
       c.component_count, c.community, c.index_token
FROM tokens t LEFT JOIN token_index_configs c ON c.id = t.id
WHERE t.version = 14 ORDER BY t.creation_block, t.creation_log_index;
SELECT token, count(*), sum(target_weight)
FROM token_components GROUP BY token HAVING sum(target_weight) <> 10000;
SELECT p.id, p.total_claimed,
       COALESCE(sum(o.amount),0) AS event_claimed
FROM walnut_pools p LEFT JOIN walnut_operations o ON o.pool=p.id AND o.operation_type='TRADECLAIM'
WHERE p.pool_type='TRADE_CURATION' GROUP BY p.id, p.total_claimed
HAVING p.total_claimed <> COALESCE(sum(o.amount),0);
SELECT count(*) AS incorrectly_staked_trade_pools FROM walnut_pools
WHERE pool_type='TRADE_CURATION' AND (total_amount<>0 OR tvl<>0 OR stakers_count<>0);
SELECT count(*) AS negative_buyback_reserves FROM token_buyback_states WHERE native_reserve<0;
SELECT community,count(*) AS distinct_users FROM rh_read.walnut_account_communities GROUP BY community;
COMMIT;
