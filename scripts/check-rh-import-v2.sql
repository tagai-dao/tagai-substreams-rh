\set ON_ERROR_STOP on
SELECT c.relname AS table_name, EXISTS (
 SELECT 1 FROM pg_constraint p WHERE p.conrelid=c.oid AND p.contype='p'
) AS has_primary_key FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
WHERE n.nspname='public' AND c.relname IN ('import_v2_events','import_v2_markets','import_v2_trade_events') ORDER BY c.relname;
SELECT kind,source,COUNT(*) AS events,MIN(block_number) AS first_block,MAX(block_number) AS last_block
FROM import_v2_events GROUP BY kind,source ORDER BY source,kind;
-- Expect four after the unified sink has crossed 86448655. Before that, zero is expected.
SELECT COUNT(*) AS new_helper_deployment_events FROM import_v2_events
WHERE block_number=86448655 AND source='0x70ed7c79d80dede1828a09b3c526df283111cb82'
AND transaction_hash='c81cbd3a01a156a3697322fc64878f717597bd8de364f0f3533098023e0a44b0';
SELECT COUNT(*) AS markets FROM import_v2_markets;
SELECT COUNT(*) AS trades,COUNT(DISTINCT (transaction_hash,call_index)) AS unique_trades FROM import_v2_trade_events;
-- Any result here is a validation failure: trade rows must match the authoritative Wrapper payload.
SELECT t.id FROM import_v2_trade_events t LEFT JOIN import_v2_events e ON e.id=t.id
WHERE e.id IS NULL OR e.kind<>'ImportedTokenTrade' OR e.source<>'0x53e65de68a0eb7f3662579f44eb9849ae5ca44ab'
 OR t.token_amount IS DISTINCT FROM (e.payload::jsonb->>'tokenAmount')::numeric
 OR t.native_amount IS DISTINCT FROM (e.payload::jsonb->>'grossNativeAmount')::numeric
 OR t.token IS DISTINCT FROM (e.payload::jsonb->>'token') OR t.recipient IS DISTINCT FROM (e.payload::jsonb->>'recipient')
 OR t.block_hash<>e.block_hash OR t.block_number<>e.block_number;
