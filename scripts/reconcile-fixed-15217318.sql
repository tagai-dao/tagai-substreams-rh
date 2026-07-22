\set ON_ERROR_STOP on

-- Acceptance baseline captured from the legacy RH Graph deployment at block
-- 15,217,318 (hash 0x8950916b...a816). The deployment was later grafted at
-- 15,268,047; at block 15,269,049 all counters and summaries were unchanged,
-- confirming there were no intervening TipTag events. Run this only against a
-- sink stopped with the exclusive range end 15,217,319.
BEGIN;

CREATE TEMP TABLE expected_counts (
    entity TEXT PRIMARY KEY,
    expected_rows BIGINT NOT NULL,
    expected_max_index BIGINT NOT NULL
) ON COMMIT DROP;

INSERT INTO expected_counts (entity, expected_rows, expected_max_index) VALUES
    ('tokens', 5, 5),
    ('token_trades', 2386, 2386),
    ('token_transfers', 3012, 3012),
    ('listed_tokens', 1, 1),
    ('ipshare_trades', 2390, 2390),
    ('value_captures', 2386, 2386),
    ('stakes', 11, 11),
    ('walnut_communities', 7, 7),
    ('walnut_pools', 7, 7),
    ('walnut_operations', 33, 33),
    ('accounts', 709, 707),
    ('token_holders', 783, 783);

CREATE TEMP VIEW actual_counts AS
SELECT 'tokens'::TEXT AS entity, count(*)::BIGINT AS actual_rows,
       coalesce(max(entity_index), 0)::BIGINT AS actual_max_index FROM tokens
UNION ALL SELECT 'token_trades', count(*), coalesce(max(entity_index), 0) FROM token_trade_events
UNION ALL SELECT 'token_transfers', count(*), coalesce(max(entity_index), 0) FROM token_transfer_events
UNION ALL SELECT 'listed_tokens', count(*), coalesce(max(entity_index), 0) FROM token_listings
UNION ALL SELECT 'ipshare_trades', count(*), coalesce(max(entity_index), 0) FROM ipshare_trade_events
UNION ALL SELECT 'value_captures', count(*), coalesce(max(entity_index), 0) FROM ipshare_value_capture_events
UNION ALL SELECT 'stakes', count(*), coalesce(max(entity_index), 0) FROM ipshare_stake_events
UNION ALL SELECT 'walnut_communities', count(*), coalesce(max(entity_index), 0) FROM walnut_communities
UNION ALL SELECT 'walnut_pools', count(*), coalesce(max(entity_index), 0) FROM walnut_pools
UNION ALL SELECT 'walnut_operations', count(*), coalesce(max(entity_index), 0) FROM walnut_operations
UNION ALL SELECT 'accounts', count(*), coalesce(max(entity_index), 0) FROM accounts
UNION ALL SELECT 'token_holders', count(*), coalesce(max(entity_index), 0) FROM token_balances;

SELECT e.entity, e.expected_rows, a.actual_rows,
       e.expected_max_index, a.actual_max_index,
       e.expected_rows = a.actual_rows
           AND e.expected_max_index = a.actual_max_index AS matches
FROM expected_counts e
JOIN actual_counts a USING (entity)
ORDER BY e.entity;

DO $$
DECLARE
    mismatch TEXT;
BEGIN
    SELECT string_agg(
        format('%s rows %s/%s max-index %s/%s', e.entity,
               a.actual_rows, e.expected_rows,
               a.actual_max_index, e.expected_max_index),
        '; ' ORDER BY e.entity
    ) INTO mismatch
    FROM expected_counts e
    JOIN actual_counts a USING (entity)
    WHERE e.expected_rows <> a.actual_rows
       OR e.expected_max_index <> a.actual_max_index;

    IF mismatch IS NOT NULL THEN
        RAISE EXCEPTION 'legacy count reconciliation failed: %', mismatch;
    END IF;
END
$$;

DO $$
DECLARE
    actual RECORD;
BEGIN
    SELECT * INTO actual FROM pump_summary WHERE id = 'pump';
    IF actual IS NULL
       OR actual.token_counts <> 5
       OR actual.listed_counts <> 1 THEN
        RAISE EXCEPTION 'pump summary mismatch: %', row_to_json(actual);
    END IF;

    SELECT * INTO actual FROM ipshare_summary WHERE id = 'summary';
    IF actual IS NULL
       OR actual.users_count <> 707
       OR actual.total_protocol_fee <> 12063922233403131
       OR actual.total_create_fee <> 0
       OR actual.buy_count <> 2397
       OR actual.sell_count <> 1
       OR actual.total_value_capture <> 421758949456664687 THEN
        RAISE EXCEPTION 'IPShare summary mismatch: %', row_to_json(actual);
    END IF;

    SELECT * INTO actual FROM walnut_summary WHERE id = 'walnut';
    IF actual IS NULL
       OR actual.tvl <> 0
       OR actual.total_communities <> 7
       OR actual.total_users <> 2
       OR actual.total_pools <> 7 THEN
        RAISE EXCEPTION 'Walnut summary mismatch: %', row_to_json(actual);
    END IF;

    IF (SELECT coalesce(max(ipshare_index), 0) FROM accounts) <> 8 THEN
        RAISE EXCEPTION 'account IPShare cursor mismatch';
    END IF;

    -- Legacy Graph has 17 Holder rows but only 16 unique (holder, subject)
    -- pairs. Its Create handler concatenates hex strings while Trade
    -- concatenates bytes, producing two IDs for the same self-holding. SQL
    -- intentionally normalizes the relation by its semantic composite key.
    IF (SELECT count(*) FROM ipshare_holders) <> 16 THEN
        RAISE EXCEPTION 'IPShare holder relationship count mismatch';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM ipshare_holders
        WHERE holder = '0x16290796f2cd9f3ee97d3dd6bddfe9557c6d9b67'
          AND subject = '0x16290796f2cd9f3ee97d3dd6bddfe9557c6d9b67'
          AND shares_owned = 6105416018915426816
    ) THEN
        RAISE EXCEPTION 'normalized self-holding balance mismatch';
    END IF;

    IF (SELECT count(*) FROM ipshare_stakers) <> 10 THEN
        RAISE EXCEPTION 'IPShare staker relationship count mismatch';
    END IF;
END
$$;

-- Known Token acceptance fixture. The two fee totals intentionally use the
-- sum of the 365 immutable legacy Trade rows (the correct cumulative value),
-- rather than the legacy Token.sellsmanFee field whose V9 mapper overwrites
-- prior fees on sells.
DO $$
DECLARE
    actual RECORD;
BEGIN
    SELECT * INTO actual
    FROM tokens
    WHERE id = '0x99121234ed5e7de803dfba09d2e2d97048ca5318';

    IF actual IS NULL
       OR actual.entity_index <> 1
       OR actual.symbol <> 'rhtst'
       OR actual.listed
       OR actual.creator <> '0x16290796f2cd9f3ee97d3dd6bddfe9557c6d9b67'
       OR actual.pump <> '0x6c75e165e52e9c1661a75041650be2d919ee02a1'
       OR actual.version <> 9
       OR actual.buy_times <> 190
       OR actual.sell_times <> 175
       OR actual.holders_count <> 70
       OR actual.tiptag_fee <> 92914689341066131
       OR actual.sellsman_fee <> 92914689341066131
       OR actual.price <> 10447288930
       OR actual.bonding_curve_supply <> 119467980525815059750051742
       OR actual.max_bonding_curve_supply <> 632755560728863851683738580 THEN
        RAISE EXCEPTION 'known Token fixture mismatch: %', row_to_json(actual);
    END IF;

    IF (SELECT count(*) FROM token_trade_events
        WHERE token = actual.id) <> 365 THEN
        RAISE EXCEPTION 'known Token trade count mismatch';
    END IF;

    IF (SELECT count(*) FROM token_balances
        WHERE token = actual.id) <> 134 THEN
        RAISE EXCEPTION 'known Token holder-row count mismatch';
    END IF;

    IF (SELECT count(*) FROM token_balances
        WHERE token = actual.id AND amount > 0) <> 70 THEN
        RAISE EXCEPTION 'known Token non-zero holder count mismatch';
    END IF;
END
$$;

-- Full Walnut field fingerprints. These cover every field consumed by the
-- legacy sync for all 7 communities, all 7 pools, and all 33 operations.
DO $$
DECLARE
    actual_md5 TEXT;
BEGIN
    SELECT md5(string_agg(
        concat(
            lower(id), '|', entity_index, '|', created_at, '|', status, '|',
            name, '|', lower(pool_factory), '|', lower(community), '|', ratio,
            '|', lower(asset), '|', chain_id, '|', lock_duration, '|',
            pool_type, '|', tvl
        ), E'\n' ORDER BY entity_index
    )) INTO actual_md5 FROM walnut_pools;
    IF actual_md5 <> '378edbddb52a3c9946483d69cd6c3ca1' THEN
        RAISE EXCEPTION 'Walnut Pool field fingerprint mismatch: %', actual_md5;
    END IF;

    SELECT md5(string_agg(
        concat(
            lower(id), '|', entity_index, '|', created_at, '|', lower(owner),
            '|', lower(dao_fund), '|', fee_ratio, '|', lower(c_token), '|',
            lower(treasury), '|', distributed_c_token, '|', revenue, '|',
            retained_revenue, '|', users_count, '|', pools_count, '|',
            active_pool_count, '|', operation_count
        ), E'\n' ORDER BY entity_index
    )) INTO actual_md5 FROM walnut_communities;
    IF actual_md5 <> 'e8c87d54f6cfea58c7723ac6b6843ae0' THEN
        RAISE EXCEPTION 'Walnut Community field fingerprint mismatch: %', actual_md5;
    END IF;

    SELECT md5(string_agg(
        concat(
            entity_index, '|', operation_type, '|', lower(community), '|',
            lower(pool_factory), '|', lower(pool), '|', lower(account), '|',
            chain_id, '|', lower(asset), '|', amount, '|', social_order_id,
            '|', social_harvested, '|', block_timestamp, '|0x',
            lower(transaction_hash)
        ), E'\n' ORDER BY entity_index
    )) INTO actual_md5 FROM walnut_operations;
    IF actual_md5 <> '61a6ef042c92397bd8f581970a4da41a' THEN
        RAISE EXCEPTION 'Walnut operation field fingerprint mismatch: %', actual_md5;
    END IF;
END
$$;

-- Mid-history cursor fixture independently reconciled with the legacy Graph.
-- This catches a globally shifted cursor even when final counts happen to agree.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM token_trade_events
        WHERE entity_index = 201
          AND token = '0x99121234ed5e7de803dfba09d2e2d97048ca5318'
          AND buyer = '0x2470d75f0ffc037e2ac9ad9cfe9ee25c3df01b66'
          AND sellsman = '0x16290796f2cd9f3ee97d3dd6bddfe9557c6d9b67'
          AND NOT is_buy
          AND token_amount = 7478211889368010000000000
          AND eth_amount = 76385224392759229
          AND tiptag_fee = 229155673178277
          AND sellsman_fee = 229155673178277
          AND price = 40256172060
          AND transaction_hash = '2c05571baf58def70abfac5c6cf01fc00a37c0382c646f9218da6dad28ccb9df'
    ) THEN
        RAISE EXCEPTION 'TokenTrade index 201 fixture mismatch';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM token_transfer_events
        WHERE entity_index = 201
          AND token = '0xbdc2b230848890d1c644e78b72cf317ee9584afa'
          AND sender = '0xeaedcafd16796dcf9510f9e7ec6808ca72f8ddd9'
          AND recipient = '0xbdc2b230848890d1c644e78b72cf317ee9584afa'
          AND amount = 2741908874638910000000000
          AND transaction_hash = '0b23fb85edca2b1f30582487d189e4f865585e7dc1fb7091886b5c27502dba9a'
    ) THEN
        RAISE EXCEPTION 'TokenTransfer index 201 fixture mismatch';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM ipshare_trade_events
        WHERE entity_index = 201
          AND trader = '0x8a7b0d80fa92699ce3e5bb2c8fe404d6733796d1'
          AND subject = '0x16290796f2cd9f3ee97d3dd6bddfe9557c6d9b67'
          AND is_buy
          AND share_amount = 33707206911763630
          AND eth_amount = 229155673178277
          AND protocol_eth_amount = 5728891829456
          AND subject_eth_amount = 10312005293022
          AND supply = 25161490296615114550
          AND transaction_hash = '2c05571baf58def70abfac5c6cf01fc00a37c0382c646f9218da6dad28ccb9df'
    ) THEN
        RAISE EXCEPTION 'IPShare Trade index 201 fixture mismatch';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM ipshare_value_capture_events
        WHERE entity_index = 201
          AND subject = '0x16290796f2cd9f3ee97d3dd6bddfe9557c6d9b67'
          AND investor = '0x99121234ed5e7de803dfba09d2e2d97048ca5318'
          AND amount = 229155673178277
          AND transaction_hash = '2c05571baf58def70abfac5c6cf01fc00a37c0382c646f9218da6dad28ccb9df'
    ) THEN
        RAISE EXCEPTION 'ValueCaptured index 201 fixture mismatch';
    END IF;
END
$$;

-- Cursor columns are explicit and must be dense, because graph-data-sync.ts
-- advances each entity independently by its legacy `index` field.
DO $$
DECLARE
    gap_description TEXT;
BEGIN
    SELECT string_agg(entity, ', ' ORDER BY entity) INTO gap_description
    FROM (
        SELECT 'tokens' AS entity FROM tokens
          HAVING count(*) <> coalesce(max(entity_index), 0)
        UNION ALL SELECT 'token_trades' FROM token_trade_events
          HAVING count(*) <> coalesce(max(entity_index), 0)
        UNION ALL SELECT 'token_transfers' FROM token_transfer_events
          HAVING count(*) <> coalesce(max(entity_index), 0)
        UNION ALL SELECT 'listed_tokens' FROM token_listings
          HAVING count(*) <> coalesce(max(entity_index), 0)
        UNION ALL SELECT 'ipshare_trades' FROM ipshare_trade_events
          HAVING count(*) <> coalesce(max(entity_index), 0)
        UNION ALL SELECT 'value_captures' FROM ipshare_value_capture_events
          HAVING count(*) <> coalesce(max(entity_index), 0)
        UNION ALL SELECT 'stakes' FROM ipshare_stake_events
          HAVING count(*) <> coalesce(max(entity_index), 0)
        UNION ALL SELECT 'walnut_communities' FROM walnut_communities
          HAVING count(*) <> coalesce(max(entity_index), 0)
        UNION ALL SELECT 'walnut_operations' FROM walnut_operations
          HAVING count(*) <> coalesce(max(entity_index), 0)
    ) gaps;

    IF gap_description IS NOT NULL THEN
        RAISE EXCEPTION 'non-dense legacy cursor(s): %', gap_description;
    END IF;
END
$$;

-- Every immutable event row keeps its canonical block hash. This is separate
-- from the SQL sink cursor/history and makes reconciliation and fork audits
-- possible without consulting an RPC endpoint.
DO $$
DECLARE
    invalid_tables TEXT;
BEGIN
    SELECT string_agg(entity, ', ' ORDER BY entity) INTO invalid_tables
    FROM (
        SELECT 'pump_token_discoveries' AS entity FROM pump_token_discoveries
          HAVING count(*) FILTER (WHERE block_hash !~ '^0x[0-9a-f]{64}$') > 0
        UNION ALL SELECT 'token_trade_events' FROM token_trade_events
          HAVING count(*) FILTER (WHERE block_hash !~ '^0x[0-9a-f]{64}$') > 0
        UNION ALL SELECT 'token_transfer_events' FROM token_transfer_events
          HAVING count(*) FILTER (WHERE block_hash !~ '^0x[0-9a-f]{64}$') > 0
        UNION ALL SELECT 'token_listings' FROM token_listings
          HAVING count(*) FILTER (WHERE block_hash !~ '^0x[0-9a-f]{64}$') > 0
        UNION ALL SELECT 'ipshare_trade_events' FROM ipshare_trade_events
          HAVING count(*) FILTER (WHERE block_hash !~ '^0x[0-9a-f]{64}$') > 0
        UNION ALL SELECT 'ipshare_value_capture_events' FROM ipshare_value_capture_events
          HAVING count(*) FILTER (WHERE block_hash !~ '^0x[0-9a-f]{64}$') > 0
        UNION ALL SELECT 'ipshare_stake_events' FROM ipshare_stake_events
          HAVING count(*) FILTER (WHERE block_hash !~ '^0x[0-9a-f]{64}$') > 0
        UNION ALL SELECT 'walnut_operations' FROM walnut_operations
          HAVING count(*) FILTER (WHERE block_hash !~ '^0x[0-9a-f]{64}$') > 0
    ) invalid;

    IF invalid_tables IS NOT NULL THEN
        RAISE EXCEPTION 'invalid or missing block hash in: %', invalid_tables;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pump_token_discoveries
        WHERE block_number = 6922897
          AND block_hash = '0xbd5cfb87bb55c8d7922d20b044bbf263f342eb77ec994c6de98011ca380aad98'
    ) THEN
        RAISE EXCEPTION 'first RH event block hash fixture mismatch';
    END IF;
END
$$;

SELECT 'fixed block 15217318 reconciliation passed' AS result;
ROLLBACK;
